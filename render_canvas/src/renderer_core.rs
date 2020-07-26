use super::renderer_layer::*;
use super::renderer_worker::*;

///
/// Parts of the renderer that are shared with the workers
///
pub struct RenderCore {
    /// The definition for the layers
    pub layers: Vec<Layer>
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
}