use super::renderer_layer::*;
use super::renderer_worker::*;

use flo_render as render;
use std::mem;

///
/// Parts of the renderer that are shared with the workers
///
pub struct RenderCore {
    /// The definition for the layers
    pub layers: Vec<Layer>,

    /// The first unused vertex buffer ID
    pub unused_vertex_buffer: usize,

    /// Vertex buffers that were previously used but are now free
    pub free_vertex_buffers: Vec<usize>
}

impl RenderCore {
    ///
    /// Stores the result of a worker job in this core item
    ///
    pub fn store_job_result(&mut self, entity_ref: LayerEntityRef, render_entity: RenderEntity) {
        // TODO: check that the entity is still valid since the last time the layer or the canvas was cleared
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
        let mut vertex_action = RenderEntity::Tessellating(LayerOperation::Draw);
        mem::swap(&mut self.layers[layer_id].render_order[render_index], &mut vertex_action);

        // The action we just removed should be a vertex buffer action
        match vertex_action {
            RenderEntity::VertexBuffer(op, vertices) => {
                // Allocate a buffer
                let buffer_id = self.allocate_vertex_buffer();

                // Draw these buffers as the action at this position
                self.layers[layer_id].render_order[render_index] = RenderEntity::DrawIndexed(op, render::VertexBufferId(buffer_id), render::IndexBufferId(buffer_id), vertices.indices.len());

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