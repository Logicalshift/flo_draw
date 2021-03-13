use super::resource_ids::*;
use super::render_entity::*;
use super::renderer_core::*;

use flo_canvas as canvas;
use flo_render as render;

use ::desync::*;

use futures::prelude::*;
use futures::task::{Context, Poll};
use futures::future::{BoxFuture};

use std::pin::*;
use std::sync::*;

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
    Texture(render::TextureId, render::Matrix, bool)
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

    /// The current layer ID that we're processing
    layer_id: usize,

    /// The render entity within the layer that we're processing
    render_index: usize,

    /// Render actions waiting to be sent
    pending_stack: Vec<render::RenderAction>,

    /// The stack of operations to run when the rendering is complete (None if they've already been rendered)
    final_stack: Option<Vec<render::RenderAction>>,

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

    /// The texture to use as the eraser mask (None for no eraser texture)
    erase_mask: Maybe<render::TextureId>,

    /// The texture to use for the clip mask (None for no clip mask)
    clip_mask: Maybe<render::TextureId>,

    /// The modifier to apply to the shader, if present
    shader_modifier: Option<ShaderModifier>,

    /// The transform to apply to the rendering instructions
    transform: Option<canvas::Transform2D>,

    /// The buffers to use to render the clipping region
    clip_buffers: Option<Vec<(render::VertexBufferId, render::IndexBufferId, usize)>>
}

impl<'a> RenderStream<'a> {
    ///
    /// Creates a new render stream
    ///
    pub fn new<ProcessFuture>(core: Arc<Desync<RenderCore>>, frame_suspended: bool, processing_future: ProcessFuture, viewport_transform: canvas::Transform2D, background_vertex_buffer: render::VertexBufferId, initial_action_stack: Vec<render::RenderAction>, final_action_stack: Vec<render::RenderAction>) -> RenderStream<'a>
    where   ProcessFuture: 'a+Send+Future<Output=()> {
        RenderStream {
            core:                       core,
            frame_suspended:            frame_suspended,
            background_vertex_buffer:   background_vertex_buffer,
            processing_future:          Some(processing_future.boxed()),
            pending_stack:              initial_action_stack,
            final_stack:                Some(final_action_stack),
            viewport_transform:         viewport_transform,
            layer_id:                   0,
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
            erase_mask:         Maybe::Unknown,
            clip_mask:          Maybe::Unknown, 
            shader_modifier:    None,
            transform:          None,
            clip_buffers:       None
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
            render::RenderAction::Create1DTextureMono(DASH_TEXTURE, DASH_WIDTH),
            render::RenderAction::WriteTexture1D(DASH_TEXTURE, 0, DASH_WIDTH, Arc::new(pixels)),
            render::RenderAction::CreateMipMaps(DASH_TEXTURE)
        ]
    }

    ///
    /// Returns the render actions needed to update from the specified state to this state (in reverse order, for replaying as a render stack)
    ///
    fn update_from_state(&self, from: &RenderStreamState) -> Vec<render::RenderAction> {
        let mut updates = vec![];
        let mut reset_render_target = false;

        // If the clip buffers are different, make sure we reset the render target state (note that updates are run in reverse order!)
        if let Some(clip_buffers) = &self.clip_buffers {
            if Some(clip_buffers) != from.clip_buffers.as_ref() && clip_buffers.len() > 0 {
                reset_render_target = true;
            }
        }

        // Update the transform state
        if let Some(transform) = self.transform {
            if Some(transform) != from.transform || (self.render_target != from.render_target && self.render_target.is_some()) || reset_render_target {
                updates.push(render::RenderAction::SetTransform(transform_to_matrix(&transform)));
            }
        }

        // Update the shader we're using
        if let (Some(erase), Some(clip), Some(modifier)) = (self.erase_mask.value(), self.clip_mask.value(), &self.shader_modifier) {
            let mask_textures_changed   = Some(erase) != from.erase_mask.value() || Some(clip) != from.clip_mask.value();
            let render_target_changed   = self.render_target != from.render_target && self.render_target.is_some();
            let modifier_changed        = Some(modifier) != from.shader_modifier.as_ref();

            if mask_textures_changed || render_target_changed || reset_render_target || modifier_changed {
                // Pick the shader based on the modifier
                let shader = match modifier {
                    ShaderModifier::Simple                              => render::ShaderType::Simple { erase_texture: erase, clip_texture: clip },
                    ShaderModifier::DashPattern(_)                      => render::ShaderType::DashedLine { dash_texture: DASH_TEXTURE, erase_texture: erase, clip_texture: clip },
                    ShaderModifier::Texture(texture_id, matrix, repeat) => render::ShaderType::Texture { texture: *texture_id, texture_transform: *matrix, repeat: *repeat, erase_texture: erase, clip_texture: clip }
                };

                // Add to the updates
                updates.push(render::RenderAction::UseShader(shader));
            }

            // Generate the texture for the modifier if that's changed
            if modifier_changed {
                match modifier {
                    ShaderModifier::Simple                          => { }
                    ShaderModifier::DashPattern(new_dash_pattern)   => { updates.extend(self.generate_dash_pattern(new_dash_pattern).into_iter().rev()); }
                    ShaderModifier::Texture(_, _, _)                => { }
                }
            }
        }

        // Set the blend mode
        if let Some(blend_mode) = self.blend_mode {
            if Some(blend_mode) != from.blend_mode || (self.render_target != from.render_target && self.render_target.is_some()) || reset_render_target {
                updates.push(render::RenderAction::BlendMode(blend_mode));
            }
        }

        // Choose the render target
        if let Some(render_target) = self.render_target {
            if Some(render_target) != from.render_target || reset_render_target {
                updates.push(render::RenderAction::SelectRenderTarget(render_target));
            }
        }

        // Update the content of the clip mask render target
        if let (Some(clip_buffers), Some(transform)) = (&self.clip_buffers, self.transform) {
            if Some(clip_buffers) != from.clip_buffers.as_ref() && clip_buffers.len() > 0 {
                let render_clip_buffers = clip_buffers.iter()
                    .rev()
                    .map(|(vertices, indices, length)| render::RenderAction::DrawIndexedTriangles(*vertices, *indices, *length));

                // Render the clip buffers once the state is set up (note: actions running in reverse!)
                updates.extend(render_clip_buffers);

                // Set up to render the clip buffers
                updates.extend(vec![
                    render::RenderAction::SetTransform(transform_to_matrix(&transform)),
                    render::RenderAction::BlendMode(render::BlendMode::AllChannelAlphaSourceOver),
                    render::RenderAction::Clear(render::Rgba8([0,0,0,255])),
                    render::RenderAction::UseShader(render::ShaderType::Simple { clip_texture: None, erase_texture: None }),
                    render::RenderAction::SelectRenderTarget(CLIP_RENDER_TARGET)
                ]);
            }
        }

        updates
    }
}

impl RenderCore {
    ///
    /// Generates the rendering actions for the layer with the specified handle
    ///
    /// The render state passed in is the expected state after this rendering has completed, and is updated to be the expected state
    /// before the rendering is completed. This slightly weird arrangement is because the rendering operations are returned as a stack:
    /// ie, they'll run in reverse order.
    ///
    fn render_layer(&mut self, viewport_transform: canvas::Transform2D, layer_handle: LayerHandle, render_state: &mut RenderStreamState) -> Vec<render::RenderAction> {
        use self::RenderEntity::*;

        let core = self;

        // Render the layer in reverse order (this is a stack, so operations are run in reverse order)
        let mut render_layer_stack      = vec![];
        let mut active_transform        = canvas::Transform2D::identity();
        let mut use_erase_texture       = false;
        let mut layer                   = core.layer(layer_handle);

        render_state.transform          = Some(viewport_transform);
        render_state.blend_mode         = Some(render::BlendMode::DestinationOver);
        render_state.render_target      = Some(MAIN_RENDER_TARGET);
        render_state.erase_mask         = Maybe::None;
        render_state.clip_mask          = Maybe::None;
        render_state.clip_buffers       = Some(vec![]);
        render_state.shader_modifier    = Some(ShaderModifier::Simple);

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

                        // Items before the sprite are rendered using the 'pre-sprite' rendering
                        render_layer_stack.extend(old_state.update_from_state(render_state));

                        // ... before that, the sprite is renderered
                        render_layer_stack.extend(render_sprite);

                        // ... using its render state
                        render_layer_stack.extend(render_state.update_from_state(&old_state));

                        // Following instructions are rendered using the state before the sprite
                        *render_state           = old_state;
                    }

                    // Reborrow the layer
                    layer                   = core.layer(layer_handle);
                },

                SetTransform(new_transform) => {
                    // The new transform will apply to all the following render instructions
                    active_transform        = *new_transform;

                    // The preceding instructions should render according to the previous state
                    let old_state           = render_state.clone();
                    render_state.transform  = Some(&viewport_transform * &active_transform);

                    render_layer_stack.extend(old_state.update_from_state(render_state));
                },

                SetBlendMode(new_blend_mode) => {
                    let mut old_state   = render_state.clone();

                    if new_blend_mode == &render::BlendMode::DestinationOut {
                        // The previous state should use the eraser texture that we're abount to generate
                        if old_state.render_target == Some(MAIN_RENDER_TARGET) {
                            old_state.erase_mask = Maybe::Some(ERASE_RENDER_TEXTURE);
                        }

                        // Render to the eraser texture
                        render_state.blend_mode     = Some(render::BlendMode::AllChannelAlphaDestinationOver);
                        render_state.render_target  = Some(ERASE_RENDER_TARGET);
                        render_state.erase_mask     = Maybe::None;

                        // Flag that we're using the erase texture and it needs to be cleared for this layer
                        use_erase_texture       = true;
                    } else {
                        // Render the main buffer
                        render_state.blend_mode     = Some(*new_blend_mode);
                        render_state.render_target  = Some(MAIN_RENDER_TARGET);

                        // Use the eraser texture if one is specified
                        if use_erase_texture {
                            render_state.erase_mask = Maybe::Some(ERASE_RENDER_TEXTURE);
                        } else {
                            render_state.erase_mask = Maybe::None;
                        }
                    }

                    // Apply the old state for the preceding instructions
                    render_layer_stack.extend(old_state.update_from_state(render_state));
                },

                DrawIndexed(vertex_buffer, index_buffer, num_items) => {
                    // Draw the triangles
                    render_layer_stack.push(render::RenderAction::DrawIndexedTriangles(*vertex_buffer, *index_buffer, *num_items));
                },

                EnableClipping(vertex_buffer, index_buffer, buffer_size) => {
                    // The preceding instructions should render according to the previous state
                    let old_state               = render_state.clone();
                    render_state.clip_mask      = Maybe::Some(CLIP_RENDER_TEXTURE);
                    render_state.clip_buffers.get_or_insert_with(|| vec![]).push((*vertex_buffer, *index_buffer, *buffer_size));

                    // Apply the old state for the preceding instructions
                    render_layer_stack.extend(old_state.update_from_state(render_state));
                }

                DisableClipping => {
                    // Remove the clip mask from the state
                    let old_state               = render_state.clone();
                    render_state.clip_mask      = Maybe::None;
                    render_state.clip_buffers   = Some(vec![]);

                    // Apply the old state for the preceding instructions
                    render_layer_stack.extend(old_state.update_from_state(render_state));
                }

                SetDashPattern(dash_pattern) => {
                    // Set the shader modifier to use the dash pattern (overriding any other shader modifier)
                    let old_state               = render_state.clone();
                    if dash_pattern.len() > 0 {
                        render_state.shader_modifier = Some(ShaderModifier::DashPattern(dash_pattern.clone()));
                    } else {
                        render_state.shader_modifier = Some(ShaderModifier::Simple);
                    }

                    // Apply the old state for the preceding instructions
                    render_layer_stack.extend(old_state.update_from_state(render_state));
                }

                SetFillTexture(texture_id, matrix, repeat) => {
                    // Set the shader modifier to use the fill texture (overriding any other shader modifier)
                    let old_state               = render_state.clone();
                    render_state.shader_modifier = Some(ShaderModifier::Texture(*texture_id, *matrix, *repeat));

                    // Apply the old state for the preceding instructions
                    render_layer_stack.extend(old_state.update_from_state(render_state));
                }
            }
        }

        // Clear the erase mask if it's used on this layer
        if use_erase_texture {
            render_state.render_target.map(|render_target| {
                render_layer_stack.push(render::RenderAction::SelectRenderTarget(render_target));
            });

            render_layer_stack.push(render::RenderAction::Clear(render::Rgba8([0, 0, 0, 0])));
            render_layer_stack.push(render::RenderAction::SelectRenderTarget(ERASE_RENDER_TARGET));
        }

        // Generate a pending set of actions for the current layer
        return render_layer_stack;
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
            let background_color = [br, bg, bb, ba];

            self.pending_stack.extend(vec![
                render::RenderAction::DrawTriangles(self.background_vertex_buffer, 0..6),
                render::RenderAction::UseShader(render::ShaderType::Simple { erase_texture: None, clip_texture: None }),
                render::RenderAction::BlendMode(render::BlendMode::DestinationOver),
                render::RenderAction::SetTransform(render::Matrix::identity()),

                // Generate a full-screen quad
                render::RenderAction::CreateVertex2DBuffer(self.background_vertex_buffer, vec![
                    render::Vertex2D { pos: [-1.0, -1.0],   tex_coord: [0.0, 0.0], color: background_color },
                    render::Vertex2D { pos: [1.0, 1.0],     tex_coord: [0.0, 0.0], color: background_color },
                    render::Vertex2D { pos: [1.0, -1.0],    tex_coord: [0.0, 0.0], color: background_color },

                    render::Vertex2D { pos: [-1.0, -1.0],   tex_coord: [0.0, 0.0], color: background_color },
                    render::Vertex2D { pos: [1.0, 1.0],     tex_coord: [0.0, 0.0], color: background_color },
                    render::Vertex2D { pos: [-1.0, 1.0],    tex_coord: [0.0, 0.0], color: background_color },
                ])
            ]);
        }
    }
}

impl<'a> Stream for RenderStream<'a> {
    type Item = render::RenderAction;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<render::RenderAction>> { 
        // Return the next pending action if there is one
        if self.pending_stack.len() > 0 {
            // Note that pending is a stack, so the items are returned in reverse
            return Poll::Ready(self.pending_stack.pop());
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
                // Layers are rendered in reverse order
                self.processing_future  = None;
                self.layer_id           = self.core.sync(|core| core.layers.len());
                self.render_index       = 0;
            }
        }

        // We've generated all the vertex buffers: if frame rendering is suspended, stop here
        if self.frame_suspended {
            if let Some(final_actions) = self.final_stack.take() {
                self.pending_stack = final_actions;
                return Poll::Ready(self.pending_stack.pop());
            } else {
                return Poll::Ready(None);
            }
        }

        // We've generated all the vertex buffers: generate the instructions to render them
        let mut layer_id        = self.layer_id;
        let viewport_transform  = self.viewport_transform;

        let result              = if layer_id == 0 {
            // Stop if we've processed all the layers
            None
        } else {
            // Move to the previous layer
            layer_id -= 1;

            self.core.sync(|core| {
                // Send any pending vertex buffers, then render the layer (note that the rendering is a stack, so the vertex buffers go on the end)
                let layer_handle        = core.layers[layer_id];
                let send_vertex_buffers = core.send_vertex_buffers(layer_handle);
                let mut render_state    = RenderStreamState::new();

                let mut render_layer    = core.render_layer(viewport_transform, layer_handle, &mut render_state);
                render_layer.extend(render_state.update_from_state(&RenderStreamState::new()));
                render_layer.extend(send_vertex_buffers);

                Some(render_layer)
            })
        };

        // Update the layer ID to continue iterating
        self.layer_id       = layer_id;

        // Add the result to the pending queue
        if let Some(result) = result {
            // There are more actions to add to the pending stack
            self.pending_stack = result;
            return Poll::Ready(self.pending_stack.pop());
        } else if let Some(final_actions) = self.final_stack.take() {
            // There are no more drawing actions, but we have a set of final post-render instructions to execute
            self.pending_stack = final_actions;
            self.render_background();
            return Poll::Ready(self.pending_stack.pop());
        } else {
            // No further actions if the result was empty
            return Poll::Ready(None);
        }
    }
}

///
/// Converts a canvas transform to a rendering matrix
///
pub fn transform_to_matrix(transform: &canvas::Transform2D) -> render::Matrix {
    let canvas::Transform2D(t) = transform;

    render::Matrix([
        [t[0][0], t[0][1], 0.0, t[0][2]],
        [t[1][0], t[1][1], 0.0, t[1][2]],
        [t[2][0], t[2][1], 1.0, t[2][2]],
        [0.0,     0.0,     0.0, 1.0]
    ])
}
