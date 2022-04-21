use super::matrix::*;
use super::layer_bounds::*;
use super::resource_ids::*;
use super::render_entity::*;
use super::renderer_core::*;
use super::layer_handle::*;
use super::texture_render_request::*;
use super::texture_filter_request::*;

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

    /// The textures that need to be set up before the layers can be rendered
    setup_textures: Vec<TextureRenderRequest>,

    /// The operations to run when the rendering is complete (None if they've already been rendered)
    final_actions: Option<Vec<render::RenderAction>>,

    /// The transformation for the viewport
    viewport_transform: canvas::Transform2D,

    /// The size of the viewport
    viewport_size: render::Size2D,

    /// The region of the layer buffer that has been drawn on
    invalid_bounds: LayerBounds
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
    is_clear: Option<bool>,

    /// The region of the layer buffer that has been drawn on
    invalid_bounds: LayerBounds,

    /// The size of the viewport
    viewport_size: render::Size2D,
}

impl<'a> RenderStream<'a> {
    ///
    /// Creates a new render stream
    ///
    pub fn new<ProcessFuture>(core: Arc<Desync<RenderCore>>, processing_future: ProcessFuture, viewport_transform: canvas::Transform2D, viewport_size: render::Size2D, background_vertex_buffer: render::VertexBufferId, initial_actions: Vec<render::RenderAction>, final_actions: Vec<render::RenderAction>) -> RenderStream<'a>
    where   ProcessFuture: 'a+Send+Future<Output=()> {
        RenderStream {
            core:                       core,
            frame_suspended:            false,
            background_vertex_buffer:   background_vertex_buffer,
            processing_future:          Some(processing_future.boxed()),
            pending:                    VecDeque::from(initial_actions),
            setup_textures:             vec![],
            final_actions:              Some(final_actions),
            viewport_transform:         viewport_transform,
            viewport_size:              viewport_size,
            layer_buffer_is_clear:      true,
            invalid_bounds:             LayerBounds::default(),
            layer_id:                   0,
            layer_count:                0,
            render_index:               0,
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
    fn new(viewport_size: render::Size2D) -> RenderStreamState {
        RenderStreamState {
            render_target:      None,
            blend_mode:         None,
            clip_mask:          Maybe::Unknown, 
            shader_modifier:    None,
            transform:          None,
            clip_buffers:       None,
            is_clear:           None,
            viewport_size:      viewport_size,
            invalid_bounds:     LayerBounds::default()
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
    /// Draws some bounds using viewport coordinates
    ///
    fn render_debug_region(&mut self, active_transform: canvas::Transform2D, viewport_size: render::Size2D, debug_region: LayerBounds, invalid_bounds: &mut LayerBounds) -> Vec<render::RenderAction> {
        // Reverse the active transform (so we figure out coordinates that will map to the debug region)
        let render::Size2D(w, h)    = viewport_size;
        let reverse_transform       = active_transform.invert().unwrap();
        let w                       = w as f32;
        let h                       = h as f32;

        // Work out where the minimum and maximum coordinates are
        let min_x           = debug_region.min_x / w;
        let min_y           = debug_region.min_y / h;
        let max_x           = debug_region.max_x / w;
        let max_y           = debug_region.max_y / h;

        let min_x           = min_x * 2.0 - 1.0;
        let min_y           = min_y * 2.0 - 1.0;
        let max_x           = max_x * 2.0 - 1.0;
        let max_y           = max_y * 2.0 - 1.0;

        let (min_x, min_y)  = reverse_transform.transform_point(min_x, min_y);
        let (max_x, max_y)  = reverse_transform.transform_point(max_x, max_y);

        // Draw to a temporary vertex buffer
        use render::RenderAction::*;
        use render::{VertexBufferId, Vertex2D};

        let mut render          = vec![];
        let debug_vertex_buffer = self.allocate_vertex_buffer();

        render.push(UseShader(render::ShaderType::Simple { clip_texture: None }));
        render.push(CreateVertex2DBuffer(VertexBufferId(debug_vertex_buffer), vec![
            Vertex2D::with_pos(min_x, min_y).with_color(0.4, 0.8, 0.0, 0.6),
            Vertex2D::with_pos(min_x, max_y).with_color(0.4, 0.8, 0.0, 0.6),
            Vertex2D::with_pos(max_x, min_y).with_color(0.4, 0.8, 0.0, 0.6),

            Vertex2D::with_pos(max_x, max_y).with_color(0.4, 0.8, 0.0, 0.6),
            Vertex2D::with_pos(max_x, min_y).with_color(0.4, 0.8, 0.0, 0.6),
            Vertex2D::with_pos(min_x, max_y).with_color(0.4, 0.8, 0.0, 0.6),
        ]));
        render.push(DrawTriangles(VertexBufferId(debug_vertex_buffer), 0..6));

        // Add back to the free list after rendering
        self.free_vertex_buffer(debug_vertex_buffer);

        // Update the invalid bounds
        let region = LayerBounds { min_x, min_y, max_x, max_y };
        let region = region.transform(&active_transform);
        invalid_bounds.combine(&region);

        render
    }

    ///
    /// Generates the rendering actions for the layer with the specified handle
    ///
    fn render_layer(&mut self, viewport_transform: canvas::Transform2D, layer_handle: LayerHandle, render_target: render::RenderTargetId, render_state: &mut RenderStreamState) -> Vec<render::RenderAction> {
        use self::RenderEntity::*;

        let core                        = self;

        // Render the layer
        let mut render_order            = vec![];
        let mut active_transform        = canvas::Transform2D::identity();
        let mut layer                   = core.layer(layer_handle);
        let initial_state               = render_state.clone();
        let layer_buffer_is_clear       = initial_state.is_clear.unwrap_or(false);
        let initial_invalid_bounds      = initial_state.invalid_bounds;
        let is_sprite                   = layer.state.is_sprite;

        render_state.transform          = Some(viewport_transform);
        render_state.blend_mode         = Some(render::BlendMode::SourceOver);
        render_state.render_target      = Some(render_target);
        render_state.clip_mask          = Maybe::None;
        render_state.clip_buffers       = Some(vec![]);
        render_state.shader_modifier    = Some(ShaderModifier::Simple);
        render_state.is_clear           = Some(false);

        // Commit the layer to the render buffer if needed
        if layer.commit_before_rendering && !layer_buffer_is_clear && !initial_invalid_bounds.is_undefined() && !is_sprite {
            render_order.extend(vec![
                render::RenderAction::RenderToFrameBuffer,
                render::RenderAction::BlendMode(render::BlendMode::SourceOver),
                render::RenderAction::DrawFrameBuffer(render_target, initial_invalid_bounds.into(), render::Alpha(1.0)),

                render::RenderAction::SelectRenderTarget(render_target),
                render::RenderAction::Clear(render::Rgba8([0,0,0,0]))
            ]);

            // This resets the invalid area of the layer buffer
            render_state.invalid_bounds = LayerBounds::default();
        }

        // Chnage the invalidated region for the layer buffer
        render_state.invalid_bounds.combine(&layer.bounds.transform(&viewport_transform));

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

                    if let Some(sprite_layer_handle) = core.sprites.get(&sprite_id) {
                        let sprite_layer_handle = *sprite_layer_handle;

                        // The sprite transform is appended to the viewport transform
                        let combined_transform      = &viewport_transform * &active_transform;
                        let combined_transform      = combined_transform * sprite_transform;

                        // The items from before the sprite should be rendered using the current state
                        let old_state               = render_state.clone();

                        // Render the layer associated with the sprite
                        let render_sprite           = core.render_layer(combined_transform, sprite_layer_handle, render_target, render_state);

                        // Render the sprite
                        render_order.extend(render_sprite);

                        // Restore the state back to the state before the sprite was rendered
                        render_order.extend(old_state.update_from_state(&render_state));

                        // Following instructions are rendered using the state before the sprite (except for the invalid area)
                        let invalid_bounds          = render_state.invalid_bounds;
                        *render_state               = old_state;
                        render_state.invalid_bounds = invalid_bounds;
                        render_state.is_clear       = Some(false);
                    }

                    // Reborrow the layer
                    layer                   = core.layer(layer_handle);
                },

                RenderSpriteWithFilters(sprite_id, sprite_transform, filters) => {
                    let sprite_id           = *sprite_id;
                    let sprite_transform    = *sprite_transform;
                    let filters             = filters.clone();

                    if let Some(sprite_layer_handle) = core.sprites.get(&sprite_id) {
                        let sprite_layer_handle     = *sprite_layer_handle;

                        // Figure out the sprite size in pixels
                        let transform               = active_transform * sprite_transform;
                        let sprite_layer            = core.layer(sprite_layer_handle);

                        // The sprite bounds are in sprite coordinates, so we need to apply the active and sprite transform to get them to 
                        let sprite_bounds_normal    = sprite_layer.bounds;
                        let sprite_bounds_viewport  = sprite_bounds_normal.transform(&(viewport_transform * transform));
                        let sprite_bounds_pixels    = sprite_bounds_viewport.to_viewport_pixels(&render_state.viewport_size);

                        // Clip the sprite bounds against the viewport to get the texture bounds
                        let viewport_bounds_pixels  = LayerBounds { min_x: 0.0, min_y: 0.0, max_x: render_state.viewport_size.0 as _, max_y: render_state.viewport_size.1 as _ };
                        let texture_bounds_pixels   = sprite_bounds_pixels.clip(&viewport_bounds_pixels);

                        if let Some(texture_bounds_pixels) = texture_bounds_pixels {
                            use render::RenderAction::*;
                            use render::{VertexBufferId, ShaderType, Vertex2D};

                            // The items from before the sprite should be rendered using the current state
                            let old_state               = render_state.clone();

                            // Allocate a texture to render to
                            let texture_bounds_pixels   = texture_bounds_pixels.snap_to_pixels();
                            let temp_texture            = core.allocate_texture();
                            let texture_vertex_buffer   = core.allocate_vertex_buffer();
                            let texture_size            = render::Size2D(texture_bounds_pixels.width() as _, texture_bounds_pixels.height() as _);

                            core.texture_size.insert(temp_texture, texture_size);

                            render_order.extend(vec![
                                CreateTextureBgra(temp_texture, texture_size),
                            ]);

                            // Create a transform that maps the sprite onto coordinates for the current viewport
                            let render_transform        = viewport_transform * (active_transform * sprite_transform);
                            let render_bounds           = texture_bounds_pixels.to_viewport_coordinates(&render_state.viewport_size);

                            // Render the sprite to the texture
                            render_order.extend(core.render_layer_to_texture(temp_texture, sprite_layer_handle, render_transform, render_bounds.to_sprite_bounds()));

                            let last_transform      = render_state.transform.unwrap_or_else(|| &viewport_transform * &active_transform);

                            // Apply filters
                            filters.iter()
                                .for_each(|filter| {
                                    render_order.extend(Self::texture_filter_request(temp_texture, viewport_transform, render_state.viewport_size, filter));
                                });

                            // The texture transform maps viewport coordinates to texture coordinates
                            let texture_transform   = 
                                canvas::Transform2D::scale(1.0/render_bounds.width(), 1.0/render_bounds.height()) *
                                canvas::Transform2D::translate(-render_bounds.min_x, -render_bounds.min_y);

                            // Render the texture to the screen, then free it
                            render_order.extend(vec![
                                SetTransform(transform_to_matrix(&canvas::Transform2D::identity())),

                                CreateMipMaps(temp_texture),
                                CreateVertex2DBuffer(VertexBufferId(texture_vertex_buffer), vec![
                                    Vertex2D::with_pos(render_bounds.min_x, render_bounds.min_y).with_texture_coordinates(0.0, 0.0),
                                    Vertex2D::with_pos(render_bounds.min_x, render_bounds.max_y).with_texture_coordinates(0.0, 1.0),
                                    Vertex2D::with_pos(render_bounds.max_x, render_bounds.min_y).with_texture_coordinates(1.0, 0.0),

                                    Vertex2D::with_pos(render_bounds.min_x, render_bounds.max_y).with_texture_coordinates(0.0, 1.0),
                                    Vertex2D::with_pos(render_bounds.max_x, render_bounds.max_y).with_texture_coordinates(1.0, 1.0),
                                    Vertex2D::with_pos(render_bounds.max_x, render_bounds.min_y).with_texture_coordinates(1.0, 0.0),
                                ]),
                                UseShader(ShaderType::Texture { 
                                    texture:            temp_texture, 
                                    texture_transform:  transform_to_matrix(&texture_transform),
                                    repeat:             false,
                                    alpha:              1.0,
                                    clip_texture:       None,
                                }),
                                DrawTriangles(VertexBufferId(texture_vertex_buffer), 0..6),

                                FreeVertexBuffer(VertexBufferId(texture_vertex_buffer)),
                                FreeTexture(temp_texture),

                                SetTransform(transform_to_matrix(&last_transform)),
                                UseShader(ShaderType::Simple { clip_texture: None }),
                            ]);

                            core.free_texture(temp_texture);
                            core.free_vertex_buffer(texture_vertex_buffer);

                            // Restore the state back to the state before the sprite was rendered
                            render_state.shader_modifier    = Some(ShaderModifier::Simple);
                            render_state.clip_mask          = Maybe::None;
                            render_order.extend(old_state.update_from_state(&render_state));

                            // Following instructions are rendered using the state before the sprite (except for the invalid area)
                            let invalid_bounds          = render_state.invalid_bounds;
                            *render_state               = old_state;
                            render_state.invalid_bounds = invalid_bounds;
                            render_state.is_clear       = Some(false);
                        }
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

                    // Render to the main buffer
                    render_state.blend_mode     = Some(*new_blend_mode);
                    render_state.render_target  = Some(render_target);

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
        if layer.commit_after_rendering && !render_state.invalid_bounds.is_undefined() && !is_sprite {
            // Work out the invalid region of the current layer
            let invalid_bounds      = render_state.invalid_bounds;

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
                canvas::BlendMode::Multiply         => render::BlendMode::Multiply,
                canvas::BlendMode::Screen           => render::BlendMode::Screen,
                canvas::BlendMode::Darken           => render::BlendMode::SourceOver,
                canvas::BlendMode::Lighten          => render::BlendMode::SourceOver,
            };

            render_order.extend(vec![
                render::RenderAction::RenderToFrameBuffer,
                render::RenderAction::BlendMode(blend_mode),
                render::RenderAction::DrawFrameBuffer(render_target, invalid_bounds.into(), render::Alpha(alpha)),

                render::RenderAction::SelectRenderTarget(render_target),
                render::RenderAction::Clear(render::Rgba8([0,0,0,0]))
            ]);

            if blend_mode != render::BlendMode::SourceOver {
                render_order.push(render::RenderAction::BlendMode(render::BlendMode::SourceOver));
            }

            // The render buffer is clear after this
            render_state.is_clear       = Some(true);
            render_state.invalid_bounds = LayerBounds::default();
        }

        // Generate a pending set of actions for the current layer
        return render_order;
    }


    ///
    /// Given a texture to use as a render target, renders a layer to it
    ///
    /// This will (re)create the texture as a render target
    ///
    fn render_layer_to_texture(&mut self, texture_id: render::TextureId, layer_handle: LayerHandle, sprite_transform: canvas::Transform2D, region: canvas::SpriteBounds) -> Vec<render::RenderAction> {
        let core = self;

        // Allocate a texture and a render target for this operation
        let offscreen_texture       = core.allocate_texture();
        let offscreen_render_target = core.allocate_render_target();

        // Need to know the texture size to recreate it as a render target
        let texture_size        = core.texture_size.get(&texture_id).cloned();
        let texture_size        = if let Some(texture_size) = texture_size { texture_size } else { return vec![] };

        // Create a viewport transform for the render region (-1.0 - 1.0 will be the texture size, so we just need a transform that maps the appropriate coordinates)
        let canvas::SpriteBounds(canvas::SpritePosition(x, y), canvas::SpriteSize(w, h)) = region;

        let viewport_transform      = 
            canvas::Transform2D::scale(2.0/w, 2.0/h) *
            canvas::Transform2D::translate(-(x+(w/2.0)), -(y+h/2.0));

        // Map the viewport so the appropriate part of the sprite is visible
        let viewport_transform      = viewport_transform * sprite_transform;

        // Start by rendering to a multi-sampled texture
        let mut render_to_texture   = vec![];

        use render::RenderAction::*;
        render_to_texture.extend(vec![
            CreateRenderTarget(offscreen_render_target, offscreen_texture, texture_size, render::RenderTargetType::MultisampledTexture),
            SelectRenderTarget(offscreen_render_target),
            Clear(render::Rgba8([0, 0, 0, 0]))
        ]);

        // Sprites render using the viewport transform only (even though they have a layer transform it's not actually updated later on. See how sprite_transform is calculated in RenderSprite also)
        let mut render_state        = RenderStreamState::new(texture_size);
        render_state.render_target  = Some(offscreen_render_target);
        render_to_texture.extend(core.render_layer(viewport_transform, layer_handle, offscreen_render_target, &mut render_state));

        // Draw the multi-sample texture to a normal texture
        render_to_texture.extend(vec![
            CreateRenderTarget(RESOLVE_RENDER_TARGET, texture_id, texture_size, render::RenderTargetType::Standard),
            SelectRenderTarget(RESOLVE_RENDER_TARGET),
            Clear(render::Rgba8([0, 0, 0, 0])),
            BlendMode(render::BlendMode::SourceOver),
            SetTransform(render::Matrix::identity()),
            DrawFrameBuffer(offscreen_render_target, render::FrameBufferRegion::default(), render::Alpha(1.0)), // TODO: render_state.invalid_bounds to improve performance, but because the viewport transform is 'wrong' for sprites the invalid bounds are also 'wrong'
        ]);

        // Return to the main framebuffer and free up the render targets
        render_to_texture.extend(vec![
            SelectRenderTarget(MAIN_RENDER_TARGET),
            FreeRenderTarget(offscreen_render_target),
            FreeRenderTarget(RESOLVE_RENDER_TARGET),
            FreeTexture(offscreen_texture),
        ]);

        core.free_texture(offscreen_texture);
        core.free_render_target(offscreen_render_target);

        render_to_texture
    }

    ///
    /// Generates the render actions for a gaussian blur filter with the specified radius
    ///
    fn filter_gaussian_blur(texture_id: render::TextureId, radius_pixels_x: f32, radius_pixels_y: f32) -> Vec<render::RenderAction> {
        // Blur has no effect below a 1px radius
        if radius_pixels_x <= 1.0 { return vec![]; };
        if radius_pixels_y <= 1.0 { return vec![]; };

        // Sigma is fixed, the x and y steps are calculated from the radius
        let sigma   = 0.25;
        let x_step  = 1.0 / radius_pixels_x;
        let y_step  = 1.0 / radius_pixels_y;

        // We calculate a kernel out to 4 sigma
        let kernel_size = ((sigma / x_step) * 8.0).ceil() as usize;
        let x_filter    = if kernel_size <= 9 {
            render::TextureFilter::GaussianBlurHorizontal9(sigma, x_step)
        } else if kernel_size <= 29 {
            render::TextureFilter::GaussianBlurHorizontal29(sigma, x_step)
        } else if kernel_size <= 61 {
            render::TextureFilter::GaussianBlurHorizontal61(sigma, x_step)
        } else {
            render::TextureFilter::GaussianBlurHorizontal(sigma, x_step, kernel_size)
        };

        let kernel_size = ((sigma / y_step) * 8.0).ceil() as usize;
        let y_filter    = if kernel_size <= 9 {
            render::TextureFilter::GaussianBlurVertical9(sigma, y_step)
        } else if kernel_size <= 29 {
            render::TextureFilter::GaussianBlurVertical29(sigma, y_step)
        } else if kernel_size <= 61 {
            render::TextureFilter::GaussianBlurVertical61(sigma, y_step)
        } else {
            render::TextureFilter::GaussianBlurVertical(sigma, y_step, kernel_size)
        };

        vec![
            render::RenderAction::FilterTexture(texture_id, vec![
                x_filter,
                y_filter,
            ])
        ]
    }

    ///
    /// Applies a filter to a texture
    ///
    fn texture_filter_request(texture_id: render::TextureId, viewport_transform: canvas::Transform2D, viewport_size: render::Size2D, request: &TextureFilterRequest) -> Vec<render::RenderAction> {
        use TextureFilterRequest::*;

        match request {
            PixelBlur(radius)               => Self::filter_gaussian_blur(texture_id, *radius, *radius),
            CanvasBlur(radius, transform)   => {
                let transform   = viewport_transform * *transform;

                // Convert the radius using the transform
                let (x1, y1)    = transform.transform_point(0.0, 0.0);
                let (x2, y2)    = transform.transform_point(*radius, *radius);

                let min_x       = f32::min(x1, x2);
                let min_y       = f32::min(y1, y2);
                let max_x       = f32::max(x1, x2);
                let max_y       = f32::max(y1, y2);

                // Size relative to the framebuffer size
                let size_w      = (max_x - min_x)/2.0;
                let size_h      = (max_y - min_y)/2.0;

                let x_radius    = viewport_size.0 as f32 * size_w;
                let y_radius    = viewport_size.1 as f32 * size_h;

                // Generate the actions to apply the filter
                Self::filter_gaussian_blur(texture_id, x_radius, y_radius)
            },
        }
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

    ///
    /// Given a texture to use as a render target, renders a layer to it
    ///
    /// This will (re)create the texture as a render target
    ///
    fn render_layer_to_texture(&self, texture_id: render::TextureId, layer_handle: LayerHandle, region: canvas::SpriteBounds) -> Vec<render::RenderAction> {
        self.core.sync(move |core| {
            core.render_layer_to_texture(texture_id, layer_handle, canvas::Transform2D::identity(), region)
        })
    }

    ///
    /// Creates the rendering actions for generating a dynamic texture
    ///
    fn render_dynamic_texture(&self, texture_id: render::TextureId, layer_handle: LayerHandle, sprite_region: canvas::SpriteBounds, canvas_size: canvas::CanvasSize, transform: canvas::Transform2D) -> Vec<render::RenderAction> {
        // Convert the transform to viewport coordinates
        let transform = self.viewport_transform * transform;

        // Coordinates for the size
        let (x1, y1)    = transform.transform_point(0.0, 0.0);
        let (x2, y2)    = transform.transform_point(canvas_size.0, canvas_size.1);

        let min_x       = f32::min(x1, x2);
        let min_y       = f32::min(y1, y2);
        let max_x       = f32::max(x1, x2);
        let max_y       = f32::max(y1, y2);

        // Size relative to the framebuffer size
        let size_w      = (max_x - min_x)/2.0;
        let size_h      = (max_y - min_y)/2.0;

        let size_w      = self.viewport_size.0 as f32 * size_w;
        let size_h      = self.viewport_size.1 as f32 * size_h;

        // Set the texture size
        let size        = render::Size2D(size_w as _, size_h as _);
        self.core.sync(|core| core.texture_size.insert(texture_id, size));

        // Render to the texture
        self.render_layer_to_texture(texture_id, layer_handle, sprite_region)
    }

    ///
    /// Modifies any 'draw framebuffer' operations in the pending list so that they render only the invalid region
    ///
    fn clip_draw_framebuffer(&self, instructions: Vec<render::RenderAction>) -> Vec<render::RenderAction> {
        if self.invalid_bounds.is_undefined() {
            // Remove any 'draw frame buffer' as there's nothing to draw
            let mut instructions = instructions;
            instructions.retain(|item| {
                match item {
                    render::RenderAction::DrawFrameBuffer(_, _, _)  => false,
                    _                                               => true
                }
            });

            instructions
        } else {
            // Convert the bounds for any 'draw frame buffer' instruction to affect only the invalid bounds
            let new_bounds          = self.invalid_bounds;
            let mut instructions    = instructions;

            instructions.iter_mut()
                .for_each(|item| {
                    match item {
                        render::RenderAction::DrawFrameBuffer(target, _bounds, alpha)  => {
                            let target  = *target;
                            let alpha   = *alpha;

                            *item       = render::RenderAction::DrawFrameBuffer(target, new_bounds.into(), alpha);
                        },

                        _ => { }
                    }
                });

            instructions
        }
    }

    ///
    /// Applies a filter to a texture
    ///
    fn texture_filter_request(&self, texture_id: render::TextureId, request: &TextureFilterRequest) -> Vec<render::RenderAction> {
        RenderCore::texture_filter_request(texture_id, self.viewport_transform, self.viewport_size, request)
    }

    ///
    /// Converts a texture render request to a set of rendering actions
    ///
    fn texture_render_request(&self, request: &TextureRenderRequest) -> Vec<render::RenderAction> {
        use TextureRenderRequest::*;

        let mut render_actions = vec![];

        match request {
            CreateBlankTexture(texture_id, canvas::TextureSize(w, h), canvas::TextureFormat::Rgba) => {
                render_actions.push(render::RenderAction::CreateTextureBgra(*texture_id, render::Size2D(*w as _, *h as _)));
            }

            SetBytes(texture_id, canvas::TexturePosition(x, y), canvas::TextureSize(w, h), bytes) => {
                render_actions.push(render::RenderAction::WriteTextureData(*texture_id, render::Position2D(*x as _, *y as _), render::Position2D((x+w) as _, (y+h) as _), Arc::clone(bytes)));
            }

            CreateMipMaps(texture_id) => {
                render_actions.push(render::RenderAction::CreateMipMaps(*texture_id));
            }

            FromSprite(texture_id, layer_handle, bounds) => {
                // Ensure that the vertex buffers are available for this sprite
                let send_vertex_buffers     = self.core.sync(|core| core.send_vertex_buffers(*layer_handle));

                // Generate the instructions for rendering the contents of the sprite to a new texture
                let rendering               = self.render_layer_to_texture(*texture_id, *layer_handle, *bounds);

                render_actions.extend(send_vertex_buffers);
                render_actions.extend(rendering);
            }

            DynamicTexture(texture_id, layer_handle, bounds, size, transform, post_rendering) => {
                // Ensure that the vertex buffers are available for this sprite
                let send_vertex_buffers     = self.core.sync(|core| core.send_vertex_buffers(*layer_handle));

                // Dynamic textures differ from normal sprite rendering in that the size of the texture depends on the resolution of the canvas (at the point the dynamic request was made)
                let rendering               = self.render_dynamic_texture(*texture_id, *layer_handle, *bounds, *size, *transform);

                render_actions.extend(send_vertex_buffers);
                render_actions.extend(rendering);

                // Dynamic textures can have a set of post-processing actions applied to them (eg, filters or CreateMipMaps)
                render_actions.extend(post_rendering.iter().flat_map(|request| self.texture_render_request(request)));
            },

            CopyTexture(source_texture_id, target_texture_id) => {
                render_actions.push(render::RenderAction::CopyTexture(*source_texture_id, *target_texture_id));

                // After retiring a copy action, reduce the usage count of the source texture
                let source_texture_id = *source_texture_id;
                self.core.desync(move |core| {
                    if let Some(source_usage_count) = core.used_textures.get_mut(&source_texture_id) {
                        *source_usage_count -= 1;
                    }
                });
            },

            Filter(texture_id, filter) => {
                render_actions.extend(self.texture_filter_request(*texture_id, filter));
            }
        }

        render_actions
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
                let render::Size2D(w, h) = self.viewport_size;
                let (setup_actions, setup_textures, release_textures, rendering_suspended) = self.core.sync(move |core| 
                    (mem::take(&mut core.setup_actions), 
                        core.setup_textures((w as _, h as _)), 
                        core.free_unused_textures(), 
                        core.frame_starts > 0));

                self.setup_textures     = setup_textures;
                self.frame_suspended    = rendering_suspended;
                
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

        // After the vertex buffers are generated, we can render any sprites to textures that are pending
        if let Some(setup_texture) = self.setup_textures.pop() {
            let render_requests = self.texture_render_request(&setup_texture);
            self.pending.extend(render_requests);
        }

        if self.pending.len() > 0 {
            return Poll::Ready(self.pending.pop_front());
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
            let mut invalid_bounds          = self.invalid_bounds;
            let viewport_size               = self.viewport_size;

            let result                  = core.sync(|core| {
                // Send any pending vertex buffers, then render the layer
                let layer_handle            = core.layers[layer_id];
                let send_vertex_buffers     = core.send_vertex_buffers(layer_handle);
                let mut render_state        = RenderStreamState::new(viewport_size);
                render_state.is_clear       = Some(layer_buffer_is_clear);
                render_state.invalid_bounds = invalid_bounds;

                let mut render_layer        = VecDeque::new();

                render_layer.extend(send_vertex_buffers);
                render_layer.extend(core.render_layer(viewport_transform, layer_handle, MAIN_RENDER_TARGET, &mut render_state));
                render_layer.extend(RenderStreamState::new(viewport_size).update_from_state(&render_state));

                // The state will update to indicate if the layer buffer is clear or not for the next layer
                layer_buffer_is_clear   = render_state.is_clear.unwrap_or(false);
                invalid_bounds          = render_state.invalid_bounds;

                Some(render_layer)
            });

            // Store the new 'is clear' setting
            self.layer_buffer_is_clear  = layer_buffer_is_clear;
            self.invalid_bounds         = invalid_bounds;

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
            let final_actions = self.clip_draw_framebuffer(final_actions);
            self.pending.extend(final_actions);
            return Poll::Ready(self.pending.pop_front());
        } else {
            // No further actions if the result was empty
            return Poll::Ready(None);
        }
    }
}
