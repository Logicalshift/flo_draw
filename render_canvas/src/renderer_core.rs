use super::renderer_layer::*;
use super::renderer_sprite::*;
use super::renderer_worker::*;

use flo_canvas as canvas;
use flo_render as render;

use std::mem;
use std::collections::{HashMap};

///
/// Parts of the renderer that are shared with the workers
///
pub struct RenderCore {
    /// The definition for the layers
    pub layers: Vec<Layer>,

    /// The definition for the sprites
    pub sprites: HashMap<canvas::SpriteId, Sprite>,

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
        // TODO: if we do nothing, we need to return the entity's vertex buffers to the free pool

        // Do nothing if the layer no longer exists
        if self.layers.len() <= entity_ref.layer_id {
            return;
        }

        // Do nothing if the entity index no longer exists
        if self.layers[entity_ref.layer_id].render_order.len() <= entity_ref.entity_index {
            return;
        }

        // The existing entity should be a 'tessellating' entry that matches the entity_ref ID
        let entity = &mut self.layers[entity_ref.layer_id].render_order[entity_ref.entity_index];
        if let RenderEntity::Tessellating(entity_id) = entity {
            if *entity_id != entity_ref.entity_id {
                return;
            }
        } else {
            return;
        }

        // Store the render entity
        self.layers[entity_ref.layer_id]
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
    pub fn send_vertex_buffer(&mut self, layer_id: usize, render_index: usize) -> Vec<render::RenderAction> {
        // Remove the action from the layer
        let mut vertex_action = RenderEntity::Missing;
        mem::swap(&mut self.layers[layer_id].render_order[render_index], &mut vertex_action);

        // The action we just removed should be a vertex buffer action
        match vertex_action {
            RenderEntity::VertexBuffer(vertices) => {
                // Allocate a buffer
                let buffer_id = self.allocate_vertex_buffer();

                // Draw these buffers as the action at this position
                self.layers[layer_id].render_order[render_index] = RenderEntity::DrawIndexed(render::VertexBufferId(buffer_id), render::IndexBufferId(buffer_id), vertices.indices.len());

                // Send the vertices and indices to the rendering engine
                vec![
                    render::RenderAction::CreateIndexBuffer(render::IndexBufferId(buffer_id), vertices.indices),
                    render::RenderAction::CreateVertex2DBuffer(render::VertexBufferId(buffer_id), vertices.vertices),
                ]
            }

            _ => panic!("send_vertex_buffer must be used on a vertex buffer item")
        }
    }
}