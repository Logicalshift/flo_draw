use super::fill_state::*;
use super::layer_state::*;
use super::layer_bounds::*;
use super::layer_handle::*;
use super::render_entity::*;
use super::render_texture::*;
use super::renderer_layer::*;
use super::render_gradient::*;
use super::renderer_worker::*;
use super::stroke_settings::*;
use super::render_entity_details::*;
use super::dynamic_texture_state::*;
use super::texture_render_request::*;

use flo_canvas as canvas;
use flo_render as render;

use lyon::tessellation::{FillRule};

use std::mem;
use std::sync::*;
use std::collections::{HashMap};

///
/// Parts of the renderer that are shared with the workers
///
pub struct RenderCore {
    /// Number of times that StartFrame has been specified without a corresponding ShowFrame
    pub frame_starts: usize,

    /// One-time setup actions that are waiting to be rendered
    pub setup_actions: Vec<render::RenderAction>,

    /// The definition for the layers
    pub layers: Vec<LayerHandle>,

    /// The background colour to clear to when rendering the canvas
    pub background_color: render::Rgba8,

    /// The definition for the sprites
    pub sprites: HashMap<canvas::SpriteId, LayerHandle>,

    /// The number of times each render texture is being used by the layers or by the canvas itself (0 = ready to free)
    pub used_textures: HashMap<render::TextureId, usize>,

    /// If a texture was created to use as a render target, this provides the render target ID
    pub render_target_for_texture: HashMap<render::TextureId, render::RenderTargetId>,

    /// The size of the textures (when in use)
    pub texture_size: HashMap<render::TextureId, render::Size2D>,

    /// The canvas transform applied to the texture, if it's a dynamic texture
    pub texture_transform: HashMap<render::TextureId, canvas::Transform2D>,

    /// The render state that a dynamic texture was last rendered at
    pub dynamic_texture_state: HashMap<render::TextureId, DynamicTextureState>,

    /// Textures that are waiting to be rendered from layers
    pub layer_textures: Vec<(render::TextureId, TextureRenderRequest)>,

    /// Maps canvas textures to render textures
    pub canvas_textures: HashMap<canvas::TextureId, RenderTexture>,

    /// Maps canvas gradients to render gradients
    pub canvas_gradients: HashMap<canvas::GradientId, RenderGradient>,

    /// The alpha value to use for each texture, next time it's used
    pub texture_alpha: HashMap<canvas::TextureId, f32>,

    /// The actual layer definitions
    pub layer_definitions: Vec<Layer>,

    /// Available layer handles
    pub free_layers: Vec<LayerHandle>,

    /// The first unused vertex buffer ID
    pub unused_vertex_buffer: usize,

    /// Vertex buffers that were previously used but are now free
    pub free_vertex_buffers: Vec<usize>,

    /// The first unused texture ID
    pub unused_texture_id: usize,

    /// Textures that were previously used but are now free
    pub free_textures: Vec<render::TextureId>
}

impl RenderCore {
    ///
    /// Frees all entities from an existing layer
    ///
    pub fn free_layer_entities(&mut self, mut layer: Layer) {
        for entity in layer.render_order.drain(..) {
            self.free_entity(entity);
        }
    }

    ///
    /// Adds the resources used by a render entity to the free pool
    ///
    pub fn free_entity(&mut self, render_entity: RenderEntity) {
        use self::RenderEntity::*;

        match render_entity {
            Missing                                 => { }
            Tessellating(_entity_id)                => { }
            VertexBuffer(_buffers, _)               => { }
            SetTransform(_)                         => { }
            SetBlendMode(_)                         => { }
            SetFlatColor                            => { }
            SetDashPattern(_)                       => { }
            RenderSprite(_, _)                      => { }
            RenderSpriteWithFilters(_, _, _)        => { }
            DisableClipping                         => { }

            SetFillTexture(texture_id, _, _, _)     => { 
                self.used_textures.get_mut(&texture_id)
                    .map(|usage_count| *usage_count -= 1);
            }

            SetFillGradient(texture_id, _, _, _)    => { 
                self.used_textures.get_mut(&texture_id)
                    .map(|usage_count| *usage_count -= 1);
            }

            EnableClipping(render::VertexBufferId(vertex_id), render::IndexBufferId(index_id), _num_vertices)   |
            DrawIndexed(render::VertexBufferId(vertex_id), render::IndexBufferId(index_id), _num_vertices)      => {
                // Each buffer is only used by one drawing operation, so we can always free them here
                self.free_vertex_buffers.push(vertex_id);
                if index_id != vertex_id {
                    self.free_vertex_buffers.push(index_id);
                }
            }
        }
    }

    ///
    /// Finds any render textures that are not in use and marks them as freed
    ///
    pub fn free_unused_textures(&mut self) -> Vec<render::RenderAction> {
        // Collect the list of unused textures
        let unused_textures = self.used_textures.iter()
            .filter(|(_texture_id, count)| **count <= 0)
            .map(|(texture_id, _count)| *texture_id)
            .collect::<Vec<_>>();

        // Free them
        let mut render_actions = vec![];

        for free_texture_id in unused_textures.into_iter() {
            // Remove from the 'used textures' hash
            self.used_textures.remove(&free_texture_id);

            // Prevent any rendering to this texture
            self.layer_textures.retain(|(id, _req)| id != &free_texture_id);

            // Free the resources attached to the texture
            self.dynamic_texture_state.remove(&free_texture_id);
            self.texture_size.remove(&free_texture_id);
            self.texture_transform.remove(&free_texture_id);

            // Add as a texture ID we can reallocate
            self.free_textures.push(free_texture_id);

            // Free the render target if there is one
            if let Some(render_target_id) = self.render_target_for_texture.get(&free_texture_id) {
                render_actions.push(render::RenderAction::FreeRenderTarget(*render_target_id));
            }

            self.render_target_for_texture.remove(&free_texture_id);

            // Generate a 'free texture' action to release the graphics memory used by this texture
            render_actions.push(render::RenderAction::FreeTexture(free_texture_id));
        }

        render_actions
    }

    ///
    /// Stores the result of a worker job in this core item
    ///
    pub fn store_job_result(&mut self, entity_ref: LayerEntityRef, render_entity: RenderEntity, details: RenderEntityDetails) {
        let LayerHandle(layer_idx)  = entity_ref.layer_id;
        let layer_idx               = layer_idx as usize;

        // Do nothing if the layer no longer exists
        if self.layer_definitions.len() <= layer_idx {
            self.free_entity(render_entity);
            return;
        }

        // Do nothing if the entity index no longer exists
        if self.layer_definitions[layer_idx].render_order.len() <= entity_ref.entity_index {
            self.free_entity(render_entity);
            return;
        }

        // The existing entity should be a 'tessellating' entry that matches the entity_ref ID
        let entity = &mut self.layer_definitions[layer_idx].render_order[entity_ref.entity_index];
        if let RenderEntity::Tessellating(entity_id) = entity {
            if *entity_id != entity_ref.entity_id {
                self.free_entity(render_entity);
                return;
            }
        } else {
            return;
        }

        // Store the render entity
        let layer = &mut self.layer_definitions[layer_idx];

        layer.render_order[entity_ref.entity_index] = render_entity;
        layer.bounds.add_entity_with_details(details);
    }

    ///
    /// Allocates a free vertex buffer ID
    ///
    pub fn allocate_vertex_buffer(&mut self) -> usize {
        self.free_vertex_buffers.pop()
            .unwrap_or_else(|| {
                let buffer_id = self.unused_vertex_buffer;
                self.unused_vertex_buffer += 1;
                buffer_id
            })
    }

    ///
    /// Allocates a texture ID
    ///
    pub fn allocate_texture(&mut self) -> render::TextureId {
        self.free_textures.pop()
            .unwrap_or_else(|| {
                let texture_id = self.unused_texture_id;
                self.unused_texture_id += 1;
                render::TextureId(texture_id)
            })
    }

    ///
    /// Returns the render actions required to send a vertex buffer (as a stack, so in reverse order)
    ///
    pub fn send_layer_vertex_buffer(&mut self, layer_id: LayerHandle, render_index: usize) -> Vec<render::RenderAction> {
        let LayerHandle(layer_idx)  = layer_id;
        let layer_idx               = layer_idx as usize;

        // Remove the action from the layer (so we don't send the same buffer again)
        let mut vertex_action = RenderEntity::Missing;
        mem::swap(&mut self.layer_definitions[layer_idx].render_order[render_index], &mut vertex_action);

        // The action we just removed should be a vertex buffer action
        match vertex_action {
            RenderEntity::VertexBuffer(vertices, intent) => {
                // Allocate a buffer
                let buffer_id = self.allocate_vertex_buffer();

                // Draw these buffers as the action at this position
                match intent {
                    VertexBufferIntent::Draw    => {
                        self.layer_definitions[layer_idx].render_order[render_index] = RenderEntity::DrawIndexed(render::VertexBufferId(buffer_id), render::IndexBufferId(buffer_id), vertices.indices.len());
                    }

                    VertexBufferIntent::Clip    => {
                        self.layer_definitions[layer_idx].render_order[render_index] = RenderEntity::EnableClipping(render::VertexBufferId(buffer_id), render::IndexBufferId(buffer_id), vertices.indices.len());
                    }
                }

                // Send the vertices and indices to the rendering engine
                vec![
                    render::RenderAction::CreateVertex2DBuffer(render::VertexBufferId(buffer_id), vertices.vertices),
                    render::RenderAction::CreateIndexBuffer(render::IndexBufferId(buffer_id), vertices.indices),
                ]
            }

            _ => panic!("send_vertex_buffer must be used on a vertex buffer item")
        }
    }

    ///
    /// Returns the render actions needed to prepare the render buffers for the specified layer (and updates the layer
    /// so that the buffers are not sent again)
    ///
    pub fn send_vertex_buffers(&mut self, layer_handle: LayerHandle) -> Vec<render::RenderAction> {
        use self::RenderEntity::*;

        let mut send_vertex_buffers = vec![];
        let mut layer               = self.layer(layer_handle);

        for render_idx in 0..layer.render_order.len() {
            match &layer.render_order[render_idx] {
                VertexBuffer(_buffers, _) => { 
                    send_vertex_buffers.extend(self.send_layer_vertex_buffer(layer_handle, render_idx)); 
                    layer = self.layer(layer_handle);
                },

                RenderSprite(sprite_id, transform)                  |
                RenderSpriteWithFilters(sprite_id, transform, _)    => { 
                    let sprite_id           = *sprite_id;
                    let transform           = *transform;
                    let sprite_layer_handle = self.sprites.get(&sprite_id).cloned();
                    let mut sprite_bounds   = LayerBounds::default();

                    if let Some(sprite_layer_handle) = sprite_layer_handle {
                        send_vertex_buffers.extend(self.send_vertex_buffers(sprite_layer_handle));

                        let sprite_layer    = self.layer(sprite_layer_handle);
                        sprite_bounds       = sprite_layer.bounds;
                        sprite_bounds.transform(&transform);
                    }

                    layer = self.layer(layer_handle);
                    layer.bounds.combine(&sprite_bounds);
                },

                _ => { }
            }
        }

        send_vertex_buffers
    }

    ///
    /// Returns a render texture for a canvas texture
    ///
    pub fn texture_for_rendering(&mut self, texture_id: canvas::TextureId) -> Option<render::TextureId> {
        // 'Ready' textures are set up for rendering: 'Loading' textures need to be finished to render
        match self.canvas_textures.get(&texture_id)? {
            RenderTexture::Ready(render_texture)    => Some(*render_texture),
            RenderTexture::Loading(render_texture)  => {
                let render_texture = *render_texture;

                // Finish the texture
                self.layer_textures.push((render_texture, TextureRenderRequest::CreateMipMaps(render_texture)));

                // Mark as finished
                self.canvas_textures.get_mut(&texture_id)
                    .map(|texture| *texture = RenderTexture::Ready(render_texture));

                Some(render_texture)
            }
        }
    }

    ///
    /// Returns a (1D) render texture for a canvas gradient
    ///
    pub fn gradient_for_rendering(&mut self, gradient_id: canvas::GradientId) -> Option<render::TextureId> {
        match self.canvas_gradients.get(&gradient_id)? {
            RenderGradient::Ready(gradient_texture, _)  => Some(*gradient_texture),
            RenderGradient::Defined(definition)         => {
                // Define a new texture
                let definition  = definition.clone();
                let texture_id  = self.allocate_texture();

                // Starts at a usage count of 0
                self.used_textures.insert(texture_id, 0);

                // Get the bytes for this gradient
                let bytes       = canvas::gradient_scale::<_, 256>(definition.clone());
                let bytes       = bytes.iter().flatten().cloned().collect::<Vec<_>>();

                // Define as a 1D texture
                self.setup_actions.extend(vec![
                    render::RenderAction::Create1DTextureBgra(texture_id, render::Size1D(256)),
                    render::RenderAction::WriteTexture1D(texture_id, render::Position1D(0), render::Position1D(256), Arc::new(bytes)),
                    render::RenderAction::CreateMipMaps(texture_id)
                ]);

                // Update the texture to 'ready'
                self.canvas_gradients.insert(gradient_id, RenderGradient::Ready(texture_id, definition));

                // The new texture is the one that will be used for rendering
                Some(texture_id)
            }
        }
    }

    ///
    /// Allocates a new layer handle to a blank layer
    ///
    pub fn allocate_layer_handle(&mut self, layer: Layer) -> LayerHandle {
        if let Some(LayerHandle(idx)) = self.free_layers.pop() {
            // Overwrite the existing layer with the new layer
            self.layer_definitions[idx as usize] = layer;
            LayerHandle(idx)
        } else {
            // Define a new layer
            self.layer_definitions.push(layer);
            LayerHandle((self.layer_definitions.len()-1) as u64)
        }
    }

    ///
    /// Releases a layer from the core (returning the layer that had this handle)
    ///
    pub fn release_layer_handle(&mut self, layer_handle: LayerHandle) -> Layer {
        // Swap in an old layer for the new layer
        let LayerHandle(layer_idx)  = layer_handle;
        let mut old_layer           = Layer {
            render_order:               vec![RenderEntity::SetTransform(canvas::Transform2D::identity())],
            state:                      LayerState {
                is_sprite:          false,
                modification_count: self.layer_definitions[layer_idx as usize].state.modification_count,
                fill_color:         FillState::Color(render::Rgba8([0, 0, 0, 255])),
                winding_rule:       FillRule::NonZero,
                stroke_settings:    StrokeSettings::new(),
                current_matrix:     canvas::Transform2D::identity(),
                sprite_matrix:      canvas::Transform2D::identity(),
                scale_factor:       1.0,
                blend_mode:         canvas::BlendMode::SourceOver,
                restore_point:      None
            },
            bounds:                     LayerBounds::default(),
            stored_states:              vec![],
            commit_before_rendering:    false,
            commit_after_rendering:     false,
            blend_mode:                 canvas::BlendMode::SourceOver,
            alpha:                      1.0
        };

        mem::swap(&mut old_layer, &mut self.layer_definitions[layer_idx as usize]);

        // Add the handle to the list of free layer handles
        self.free_layers.push(layer_handle);

        // Result is the layer that was released
        old_layer
    }

    ///
    /// Returns a reference to the layer with the specified handle
    ///
    #[inline] pub fn layer(&mut self, layer_handle: LayerHandle) -> &mut Layer {
        let LayerHandle(layer_idx)  = layer_handle;
        let layer_idx               = layer_idx as usize;

        &mut self.layer_definitions[layer_idx]
    }
}
