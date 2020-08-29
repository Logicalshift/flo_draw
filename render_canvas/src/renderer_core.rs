use super::layer_state::*;
use super::render_entity::*;
use super::renderer_layer::*;
use super::renderer_worker::*;
use super::stroke_settings::*;

use flo_canvas as canvas;
use flo_render as render;

use std::mem;
use std::collections::{HashMap};

///
/// Handle referencing a renderer layer
///
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct LayerHandle(u64);

///
/// Parts of the renderer that are shared with the workers
///
pub struct RenderCore {
    /// The definition for the layers
    pub layers: Vec<LayerHandle>,

    /// The definition for the sprites
    pub sprites: HashMap<canvas::SpriteId, LayerHandle>,

    /// The actual layer definitions
    pub layer_definitions: Vec<Layer>,

    /// Available layer handles
    pub free_layers: Vec<LayerHandle>,

    /// The first unused vertex buffer ID
    pub unused_vertex_buffer: usize,

    /// Vertex buffers that were previously used but are now free
    pub free_vertex_buffers: Vec<usize>
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
            Missing                         => { }
            Tessellating(_entity_id)        => { }
            VertexBuffer(_buffers)          => { }
            SetTransform(_)                 => { }
            SetBlendMode(_)                 => { }
            RenderSprite(_, _)              => { }

            DrawIndexed(render::VertexBufferId(vertex_id), render::IndexBufferId(index_id), _num_vertices) => {
                // Each buffer is only used by one drawing operation, so we can always free them here
                self.free_vertex_buffers.push(vertex_id);
                if index_id != vertex_id {
                    self.free_vertex_buffers.push(index_id);
                }
            }
        }
    }

    ///
    /// Stores the result of a worker job in this core item
    ///
    pub fn store_job_result(&mut self, entity_ref: LayerEntityRef, render_entity: RenderEntity) {
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
        self.layer_definitions[layer_idx]
            .render_order[entity_ref.entity_index] = render_entity;
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
    /// Returns the render actions required to send a vertex buffer (as a stack, so in reverse order)
    ///
    pub fn send_layer_vertex_buffer(&mut self, layer_id: LayerHandle, render_index: usize) -> Vec<render::RenderAction> {
        let LayerHandle(layer_idx)  = layer_id;
        let layer_idx               = layer_idx as usize;

        // Remove the action from the layer
        let mut vertex_action = RenderEntity::Missing;
        mem::swap(&mut self.layer_definitions[layer_idx].render_order[render_index], &mut vertex_action);

        // The action we just removed should be a vertex buffer action
        match vertex_action {
            RenderEntity::VertexBuffer(vertices) => {
                // Allocate a buffer
                let buffer_id = self.allocate_vertex_buffer();

                // Draw these buffers as the action at this position
                self.layer_definitions[layer_idx].render_order[render_index] = RenderEntity::DrawIndexed(render::VertexBufferId(buffer_id), render::IndexBufferId(buffer_id), vertices.indices.len());

                // Send the vertices and indices to the rendering engine
                vec![
                    render::RenderAction::CreateIndexBuffer(render::IndexBufferId(buffer_id), vertices.indices),
                    render::RenderAction::CreateVertex2DBuffer(render::VertexBufferId(buffer_id), vertices.vertices),
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
                VertexBuffer(_buffers)                      => { 
                    send_vertex_buffers.extend(self.send_layer_vertex_buffer(layer_handle, render_idx)); 
                    layer = self.layer(layer_handle);
                },

                RenderSprite(sprite_id, _sprite_transform)  => { 
                    let sprite_id           = *sprite_id;
                    let sprite_layer_handle = self.sprites.get(&sprite_id).cloned();

                    if let Some(sprite_layer_handle) = sprite_layer_handle {
                        send_vertex_buffers.extend(self.send_vertex_buffers(sprite_layer_handle));
                    }

                    layer = self.layer(layer_handle);
                },

                _                                           => { }
            }
        }

        send_vertex_buffers
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
            render_order:       vec![RenderEntity::SetTransform(canvas::Transform2D::identity())],
            state:              LayerState {
                fill_color:         render::Rgba8([0, 0, 0, 255]),
                stroke_settings:    StrokeSettings::new(),
                current_matrix:     canvas::Transform2D::identity(),
                sprite_matrix:      canvas::Transform2D::identity(),
                blend_mode:         canvas::BlendMode::SourceOver,
                restore_point:      None
            },
            stored_states:      vec![]
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
