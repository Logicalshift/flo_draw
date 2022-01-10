use super::matrix::*;
use super::resource_ids::*;
use super::render_entity::*;
use super::renderer_core::*;

use flo_canvas as canvas;
use flo_render as render;

use ::desync::*;

use futures::prelude::*;
use futures::task::{Context, Poll};
use futures::future::{BoxFuture};

use std::mem;
use std::pin::*;
use std::sync::*;
use std::collections::{VecDeque};

///
/// Tri-state version of 'option' that supports 'Unknown' as well as None and Some
///
#[derive(Clone, Copy, PartialEq)]
enum Maybe<T> {
    Unknown,
    None,
    Some(T)
}

///
/// Modifier to apply to the active shader
///
#[derive(Clone, PartialEq)]
enum ShaderModifier {
    /// The simple shader should be used
    Simple,

    /// Shader should use a dash pattern
    DashPattern(Vec<f32>),

    /// Shader should use a texture
    Texture(render::TextureId, render::Matrix, bool, f32),

    /// Shader should use a gradient
    Gradient(render::TextureId, render::Matrix, bool, f32),
}

///
/// Stream of rendering actions resulting from a draw instruction
///
pub struct RenderStream<'a> {
    /// The core where the render instructions are read from
    core: Arc<Desync<RenderCore>>,

    /// The ID of the buffer to use for rendering the background quad
    background_vertex_buffer: render::VertexBufferId,

    /// True if the frame is suspended (we're not going to generate any direct rendering due to this drawing operation)
    frame_suspended: bool,

    /// The future that is processing new drawing instructions
    processing_future: Option<BoxFuture<'a, ()>>,

    /// Set to true if the layer buffer is clear after rendering the current layer
    layer_buffer_is_clear: bool,

    /// The current layer ID that we're processing
    layer_id: usize,

    /// The total number of layers in the core
    layer_count: usize,

    /// The render entity within the layer that we're processing
    render_index: usize,

    /// Render actions waiting to be sent
    pending: VecDeque<render::RenderAction>,

    /// The operations to run when the rendering is complete (None if they've already been rendered)
    final_actions: Option<Vec<render::RenderAction>>,

    /// The transformation for the viewport
    viewport_transform: canvas::Transform2D
}

///
/// Represents the active state of the render stream
///
#[derive(Clone)]
struct RenderStreamState {
    /// The render target
    render_target: Option<render::RenderTargetId>,

    /// The blend mode to use
    blend_mode: Option<render::BlendMode>,

    /// The texture to use for the clip mask (None for no clip mask)
    clip_mask: Maybe<render::TextureId>,

    /// The modifier to apply to the shader, if present
    shader_modifier: Option<ShaderModifier>,

    /// The transform to apply to the rendering instructions
    transform: Option<canvas::Transform2D>,

    /// The buffers to use to render the clipping region
    clip_buffers: Option<Vec<(render::VertexBufferId, render::IndexBufferId, usize)>>,

    /// Set to true or false if this layer has left the layer buffer clear (or None if this is unknown)
    is_clear: Option<bool>
}

impl<'a> RenderStream<'a> {
    ///
    /// Creates a new render stream
    ///
    pub fn new<ProcessFuture>(core: Arc<Desync<RenderCore>>, frame_suspended: bool, processing_future: ProcessFuture, viewport_transform: canvas::Transform2D, background_vertex_buffer: render::VertexBufferId, initial_actions: Vec<render::RenderAction>, final_actions: Vec<render::RenderAction>) -> RenderStream<'a>
    where   ProcessFuture: 'a+Send+Future<Output=()> {
        RenderStream {
            core:                       core,
            frame_suspended:            frame_suspended,
            background_vertex_buffer:   background_vertex_buffer,
            processing_future:          Some(processing_future.boxed()),
            pending:                    VecDeque::from(initial_actions),
            final_actions:              Some(final_actions),
            viewport_transform:         viewport_transform,
            layer_buffer_is_clear:      true,
            layer_id:                   0,
            layer_count:                0,
            render_index:               0
        }
    }
}

impl<T> Maybe<T> {
    ///
    /// Converts to an optional value
    ///
    pub fn value(self) -> Option<Option<T>> {
        match self {
            Maybe::Unknown      => None,
            Maybe::None         => Some(None),
            Maybe::Some(val)    => Some(Some(val))
        }
    }
}

impl RenderStreamState {
    ///
    /// Creates a new render stream state
    ///
    fn new() -> RenderStreamState {
        RenderStreamState {
            render_target:      None,
            blend_mode:         None,
            clip_mask:          Maybe::Unknown, 
            shader_modifier:    None,
            transform:          None,
            clip_buffers:       None,
            is_clear:           None
        }
    }

    ///
    /// Generates the actions required to set a particular dash pattern
    ///
    fn generate_dash_pattern(&self, pattern: &[f32]) -> Vec<render::RenderAction> {
        // Number of pixels in the dash pattern texture
        const DASH_WIDTH: usize = 256;

        // Total length determines how many bytes each dash uses
        let total_length: f32   = pattern.iter().cloned().sum();
        let pixel_length        = total_length / DASH_WIDTH as f32;

        // Do not generate a pattern for the case where the total length doesn't add up
        if total_length <= 0.0 {
            return vec![];
        }

        // Write the pixels for the dash pattern
        let mut pixels      = vec![];
        let mut pos         = 0.0;
        let mut col         = 255u8;
        let mut cur_pos     = pattern.iter();
        let mut dash_end    = *cur_pos.next().unwrap_or(&total_length);

        for _ in 0..DASH_WIDTH {
            // Switch colours while we're over the end of the dash position
            while dash_end < pos {
                let next_dash_len = cur_pos.next().unwrap_or(&total_length);
                col = if col == 0 { 255 } else { 0 };

                dash_end += next_dash_len;
            }

            // Write this pixel
            pixels.push(col);

            // Update the position
            pos += pixel_length;
        }

        // Generate the dash texture by clobbering any existing texture
        vec![
            render::RenderAction::Create1DTextureMono(DASH_TEXTURE, render::Size1D(DASH_WIDTH)),
            render::RenderAction::WriteTexture1D(DASH_TEXTURE, render::Position1D(0), render::Position1D(DASH_WIDTH), Arc::new(pixels)),
            render::RenderAction::CreateMipMaps(DASH_TEXTURE)
        ]
    }

    ///
    /// Returns the render actions needed to update from the specified state to this state
    ///
    fn update_from_state(&self, from: &RenderStreamState) -> Vec<render::RenderAction> {
        let mut updates = vec![];
        let mut reset_render_target = false;

        // Update the content of the clip mask render target
        if let (Some(clip_buffers), Some(transform)) = (&self.clip_buffers, self.transform) {
            if Some(clip_buffers) != from.clip_buffers.as_ref() && clip_buffers.len() > 0 {
                let render_clip_buffers = clip_buffers.iter()
                    .rev()
                    .map(|(vertices, indices, length)| render::RenderAction::DrawIndexedTriangles(*vertices, *indices, *length));

                // Set up to render the clip buffers
                updates.extend(vec![
                    render::RenderAction::SelectRenderTarget(CLIP_RENDER_TARGET),
                    render::RenderAction::UseShader(render::ShaderType::Simple { clip_texture: None }),
                    render::RenderAction::Clear(render::Rgba8([0,0,0,255])),
                    render::RenderAction::BlendMode(render::BlendMode::AllChannelAlphaSourceOver),
                    render::RenderAction::SetTransform(transform_to_matrix(&transform)),
                ]);

                // Render the clip buffers once the state is set up
                updates.extend(render_clip_buffers);
            }
        }

        // If the clip buffers are different, make sure we reset the render target state
        if let Some(clip_buffers) = &self.clip_buffers {
            if Some(clip_buffers) != from.clip_buffers.as_ref() && clip_buffers.len() > 0 {
                reset_render_target = true;
            }
        }

        // Choose the render target
        if let Some(render_target) = self.render_target {
            if Some(render_target) != from.render_target || reset_render_target {
                updates.push(render::RenderAction::SelectRenderTarget(render_target));
            }
        }

        // Set the blend mode
        if let Some(blend_mode) = self.blend_mode {
            if Some(blend_mode) != from.blend_mode || (self.render_target != from.render_target && self.render_target.is_some()) || reset_render_target {
                updates.push(render::RenderAction::BlendMode(blend_mode));
            }
        }

        // Update the shader we're using
        if let (Some(clip), Some(modifier)) = (self.clip_mask.value(), &self.shader_modifier) {
            let mask_textures_changed   = Some(clip) != from.clip_mask.value();
            let render_target_changed   = self.render_target != from.render_target && self.render_target.is_some();
            let modifier_changed        = Some(modifier) != from.shader_modifier.as_ref();

            if mask_textures_changed || render_target_changed || reset_render_target || modifier_changed {
                // Pick the shader based on the modifier
                let shader = match modifier {
                    ShaderModifier::Simple                                      => render::ShaderType::Simple { clip_texture: clip },
                    ShaderModifier::DashPattern(_)                              => render::ShaderType::DashedLine { dash_texture: DASH_TEXTURE, clip_texture: clip },
                    ShaderModifier::Texture(texture_id, matrix, repeat, alpha)  => render::ShaderType::Texture { texture: *texture_id, texture_transform: *matrix, repeat: *repeat, alpha: *alpha, clip_texture: clip },
                    ShaderModifier::Gradient(texture_id, matrix, repeat, alpha) => render::ShaderType::LinearGradient { texture: *texture_id, texture_transform: *matrix, repeat: *repeat, alpha: *alpha, clip_texture: clip }
                };

                // Add to the updates
                updates.push(render::RenderAction::UseShader(shader));
            }

            // Generate the texture for the modifier if that's changed
            if modifier_changed {
                match modifier {
                    ShaderModifier::Simple                          => { }
                    ShaderModifier::DashPattern(new_dash_pattern)   => { updates.extend(self.generate_dash_pattern(new_dash_pattern).into_iter().rev()); }
                    ShaderModifier::Texture(_, _, _, _)             => { }
                    ShaderModifier::Gradient(_, _, _, _)            => { }
                }
            }
        }

        // Update the transform state
        if let Some(transform) = self.transform {
            if Some(transform) != from.transform || (self.render_target != from.render_target && self.render_target.is_some()) || reset_render_target {
                updates.push(render::RenderAction::SetTransform(transform_to_matrix(&transform)));
            }
        }

        updates
    }
}

impl RenderCore {
    ///
    /// Generates the rendering actions for the layer with the specified handle
    ///
    fn render_layer(&mut self, viewport_transform: canvas::Transform2D, layer_handle: LayerHandle, render_state: &mut RenderStreamState) -> Vec<render::RenderAction> {
        use self::RenderEntity::*;

        let core                        = self;

        // Render the layer
        let mut render_order            = vec![];
        let mut active_transform        = canvas::Transform2D::identity();
        let mut layer                   = core.layer(layer_handle);
        let initial_state               = render_state.clone();
        let layer_buffer_is_clear       = initial_state.is_clear.unwrap_or(false);

        render_state.transform          = Some(viewport_transform);
        render_state.blend_mode         = Some(render::BlendMode::SourceOver);
        render_state.render_target      = Some(MAIN_RENDER_TARGET);
        render_state.clip_mask          = Maybe::None;
        render_state.clip_buffers       = Some(vec![]);
        render_state.shader_modifier    = Some(ShaderModifier::Simple);
        render_state.is_clear           = Some(false);

        // Commit the layer to the render buffer if needed
        if layer.commit_before_rendering && !layer_buffer_is_clear {
            render_order.extend(vec![
                render::RenderAction::RenderToFrameBuffer,
                render::RenderAction::BlendMode(render::BlendMode::SourceOver),
                render::RenderAction::DrawFrameBuffer(MAIN_RENDER_TARGET, render::Alpha(1.0)),

                render::RenderAction::SelectRenderTarget(MAIN_RENDER_TARGET),
                render::RenderAction::Clear(render::Rgba8([0,0,0,0]))
            ]);
        }

        // Update to the new state for this layer
        render_order.extend(render_state.update_from_state(&initial_state));

        for render_idx in 0..layer.render_order.len() {
            match &layer.render_order[render_idx] {
                Missing => {
                    // Temporary state while sending a vertex buffer?
                    panic!("Tessellation is not complete (vertex buffer went missing)");
                },

                Tessellating(_id) => { 
                    // Being processed? (shouldn't happen)
                    panic!("Tessellation is not complete (tried to render too early)");
                },

                VertexBuffer(_buffers, _) => {
                    // Should already have sent all the vertex buffers
                    panic!("Tessellation is not complete (found unexpected vertex buffer in layer)");
                },

                DrawIndexed(vertex_buffer, index_buffer, num_items) => {
                    // Draw the triangles
                    render_order.push(render::RenderAction::DrawIndexedTriangles(*vertex_buffer, *index_buffer, *num_items));
                },

                RenderSprite(sprite_id, sprite_transform) => { 
                    let sprite_id           = *sprite_id;
                    let sprite_transform    = *sprite_transform;

                    if let Some(sprite_layer) = core.sprites.get(&sprite_id) {
                        let sprite_layer = *sprite_layer;

                        // The sprite transform is appended to the viewport transform
                        let combined_transform  = &viewport_transform * &active_transform;
                        let sprite_transform    = combined_transform * sprite_transform;

                        // The items from before the sprite should be rendered using the current state
                        let old_state           = render_state.clone();

                        // Render the layer associated with the sprite
                        let render_sprite       = core.render_layer(sprite_transform, sprite_layer, render_state);

                        // Render the sprite
                        render_order.extend(render_sprite);

                        // Restore the state back to the state before the sprite was rendered
                        render_order.extend(old_state.update_from_state(&render_state));

                        // Following instructions are rendered using the state before the sprite
                        *render_state           = old_state;
                    }

                    // Reborrow the layer
                    layer                   = core.layer(layer_handle);
                },

                SetTransform(new_transform) => {
                    // The new transform will apply to all the following render instructions
                    active_transform        = *new_transform;

                    // Update the state to a state with the new transformation applied
                    let old_state           = render_state.clone();
                    render_state.transform  = Some(&viewport_transform * &active_transform);

                    render_order.extend(render_state.update_from_state(&old_state));
                },

                SetBlendMode(new_blend_mode) => {
                    let old_state               = render_state.clone();

                    // Render the main buffer
                    render_state.blend_mode     = Some(*new_blend_mode);
                    render_state.render_target  = Some(MAIN_RENDER_TARGET);

                    // Update to the new state
                    render_order.extend(render_state.update_from_state(&old_state));
                },

                EnableClipping(vertex_buffer, index_buffer, buffer_size) => {
                    // The preceding instructions should render according to the previous state
                    let old_state               = render_state.clone();
                    render_state.clip_mask      = Maybe::Some(CLIP_RENDER_TEXTURE);
                    render_state.clip_buffers.get_or_insert_with(|| vec![]).push((*vertex_buffer, *index_buffer, *buffer_size));

                    // Update to the new state
                    render_order.extend(render_state.update_from_state(&old_state));
                }

                DisableClipping => {
                    // Remove the clip mask from the state
                    let old_state               = render_state.clone();
                    render_state.clip_mask      = Maybe::None;
                    render_state.clip_buffers   = Some(vec![]);

                    // Update to the new state
                    render_order.extend(render_state.update_from_state(&old_state));
                }

                SetFlatColor => {
                    // Set the shader modifier to use the dash pattern (overriding any other shader modifier)
                    let old_state                   = render_state.clone();
                    render_state.shader_modifier    = Some(ShaderModifier::Simple);

                    // Update to the new state
                    render_order.extend(render_state.update_from_state(&old_state));
                }

                SetDashPattern(dash_pattern) => {
                    // Set the shader modifier to use the dash pattern (overriding any other shader modifier)
                    let old_state               = render_state.clone();
                    if dash_pattern.len() > 0 {
                        render_state.shader_modifier = Some(ShaderModifier::DashPattern(dash_pattern.clone()));
                    } else {
                        render_state.shader_modifier = Some(ShaderModifier::Simple);
                    }

                    // Update to the new state
                    render_order.extend(render_state.update_from_state(&old_state));
                }

                SetFillTexture(texture_id, matrix, repeat, alpha) => {
                    // Set the shader modifier to use the fill texture (overriding any other shader modifier)
                    let old_state               = render_state.clone();
                    render_state.shader_modifier = Some(ShaderModifier::Texture(*texture_id, *matrix, *repeat, *alpha));

                    // Update to the new state
                    render_order.extend(render_state.update_from_state(&old_state));
                }

                SetFillGradient(texture_id, matrix, repeat, alpha) => {
                    // Set the shader modifier to use the gradient texture (overriding any other shader modifier)
                    let old_state                   = render_state.clone();
                    render_state.shader_modifier    = Some(ShaderModifier::Gradient(*texture_id, *matrix, *repeat, *alpha));

                    // Update to the new state
                    render_order.extend(render_state.update_from_state(&old_state));
                }
            }
        }

        // If the layer has 'commit after rendering' and the next layer does not have 'commit before rendering', then commit what we just rendered
        if layer.commit_after_rendering {
            // The render buffer is clear after this
            render_state.is_clear = Some(true);

            // The blend mode for the layer
            let alpha       = layer.alpha;
            let blend_mode  = match layer.blend_mode {
                canvas::BlendMode::SourceOver       => render::BlendMode::SourceOver,
                canvas::BlendMode::SourceIn         => render::BlendMode::SourceIn,
                canvas::BlendMode::SourceOut        => render::BlendMode::SourceOut,
                canvas::BlendMode::DestinationOver  => render::BlendMode::DestinationOver,
                canvas::BlendMode::DestinationIn    => render::BlendMode::DestinationIn,
                canvas::BlendMode::DestinationOut   => render::BlendMode::DestinationOut,
                canvas::BlendMode::SourceAtop       => render::BlendMode::SourceATop,
                canvas::BlendMode::DestinationAtop  => render::BlendMode::DestinationATop,
                canvas::BlendMode::Multiply         => render::BlendMode::SourceOver,
                canvas::BlendMode::Screen           => render::BlendMode::SourceOver,
                canvas::BlendMode::Darken           => render::BlendMode::SourceOver,
                canvas::BlendMode::Lighten          => render::BlendMode::SourceOver,
            };

            render_order.extend(vec![
                render::RenderAction::RenderToFrameBuffer,
                render::RenderAction::BlendMode(blend_mode),
                render::RenderAction::DrawFrameBuffer(MAIN_RENDER_TARGET, render::Alpha(alpha)),

                render::RenderAction::SelectRenderTarget(MAIN_RENDER_TARGET),
                render::RenderAction::Clear(render::Rgba8([0,0,0,0]))
            ]);

            if blend_mode != render::BlendMode::SourceOver {
                render_order.push(render::RenderAction::BlendMode(render::BlendMode::SourceOver));
            }
        }

        // Generate a pending set of actions for the current layer
        return render_order;
    }
}

impl<'a> RenderStream<'a> {
    ///
    /// Adds the instructions required to render the background colour to the pending queue
    ///
    fn render_background(&mut self) {
        let background_color = self.core.sync(|core| core.background_color);

        // If there's a background colour, then the finalize step should draw it (the OpenGL renderer has issues blitting alpha blended multisampled textures, so this hides that the 'clear' step above doesn't work there)
        let render::Rgba8([br, bg, bb, ba]) = background_color;

        if ba > 0 {
            // Create the actions to render the background colour
            let background_color    = [br, bg, bb, ba];
            let background_actions  = vec![
                // Generate a full-screen quad
                render::RenderAction::CreateVertex2DBuffer(self.background_vertex_buffer, vec![
                    render::Vertex2D { pos: [-1.0, -1.0],   tex_coord: [0.0, 0.0], color: background_color },
                    render::Vertex2D { pos: [1.0, 1.0],     tex_coord: [0.0, 0.0], color: background_color },
                    render::Vertex2D { pos: [1.0, -1.0],    tex_coord: [0.0, 0.0], color: background_color },

                    render::Vertex2D { pos: [-1.0, -1.0],   tex_coord: [0.0, 0.0], color: background_color },
                    render::Vertex2D { pos: [1.0, 1.0],     tex_coord: [0.0, 0.0], color: background_color },
                    render::Vertex2D { pos: [-1.0, 1.0],    tex_coord: [0.0, 0.0], color: background_color },
                ]),

                // Render the quad using the default blend mode
                render::RenderAction::RenderToFrameBuffer,
                render::RenderAction::SetTransform(render::Matrix::identity()),
                render::RenderAction::BlendMode(render::BlendMode::SourceOver),
                render::RenderAction::UseShader(render::ShaderType::Simple { clip_texture: None }),
                render::RenderAction::DrawTriangles(self.background_vertex_buffer, 0..6),
            ];

            // Add to the end of the queue
            self.pending.extend(background_actions);
        }
    }
}

impl<'a> Stream for RenderStream<'a> {
    type Item = render::RenderAction;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<render::RenderAction>> { 
        // Return the next pending action if there is one
        if self.pending.len() > 0 {
            return Poll::Ready(self.pending.pop_front());
        }

        // Poll the tessellation process if it's still running
        if let Some(processing_future) = self.processing_future.as_mut() {
            // Poll the future and send over any vertex buffers that might be waiting
            if processing_future.poll_unpin(context) == Poll::Pending {
                // Still generating render buffers
                // TODO: can potentially send the buffers to the renderer when they're generated here
                return Poll::Pending;
            } else {
                // Finished processing the rendering: can send the actual rendering commands to the hardware layer
                self.processing_future  = None;
                self.layer_count        = self.core.sync(|core| core.layers.len());
                self.render_index       = 0;

                // Perform any setup actions that might exist or have been generated before proceeding
                let (setup_actions, release_textures)   = self.core.sync(|core| (mem::take(&mut core.setup_actions), core.free_unused_textures()));
                
                // TODO: would be more memory efficient to release the textures first, but it's possible for the texture setup to create and never use a texture that is then released...
                self.pending.extend(setup_actions.into_iter());
                self.pending.extend(release_textures);
                self.render_background();

                if let Some(next) = self.pending.pop_front() {
                    return Poll::Ready(Some(next));
                }
            }
        }

        // We've generated all the vertex buffers: if frame rendering is suspended, stop here
        if self.frame_suspended {
            if let Some(final_actions) = self.final_actions.take() {
                self.pending = final_actions.into();
                return Poll::Ready(self.pending.pop_front());
            } else {
                return Poll::Ready(None);
            }
        }

        // We've generated all the vertex buffers: generate the instructions to render them
        let mut layer_id        = self.layer_id;
        let viewport_transform  = self.viewport_transform;

        let result              = if layer_id >= self.layer_count {
            // Stop if we've processed all the layers
            None
        } else {
            let core                        = &self.core;
            let mut layer_buffer_is_clear   = self.layer_buffer_is_clear;

            let result                  = core.sync(|core| {
                // Send any pending vertex buffers, then render the layer
                let layer_handle        = core.layers[layer_id];
                let send_vertex_buffers = core.send_vertex_buffers(layer_handle);
                let mut render_state    = RenderStreamState::new();
                render_state.is_clear   = Some(layer_buffer_is_clear);

                let mut render_layer    = VecDeque::new();

                render_layer.extend(send_vertex_buffers);
                render_layer.extend(core.render_layer(viewport_transform, layer_handle, &mut render_state));
                render_layer.extend(RenderStreamState::new().update_from_state(&render_state));

                // The state will update to indicate if the layer buffer is clear or not for the next layer
                layer_buffer_is_clear   = render_state.is_clear.unwrap_or(false);

                Some(render_layer)
            });

            // Store the new 'is clear' setting
            self.layer_buffer_is_clear  = layer_buffer_is_clear;

            // Advance the layer ID
            layer_id += 1;

            result
        };

        // Update the layer ID to continue iterating
        self.layer_id       = layer_id;

        // Add the result to the pending queue
        if let Some(result) = result {
            // There are more actions to add to the pending actions
            self.pending = result.into();
            return Poll::Ready(self.pending.pop_front());
        } else if let Some(final_actions) = self.final_actions.take() {
            // There are no more drawing actions, but we have a set of final post-render instructions to execute
            self.pending.extend(final_actions);
            return Poll::Ready(self.pending.pop_front());
        } else {
            // No further actions if the result was empty
            return Poll::Ready(None);
        }
    }
}
