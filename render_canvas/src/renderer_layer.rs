use super::layer_state::*;
use super::render_entity::*;

use flo_canvas as canvas;

///
/// Definition of a layer in the canvas
///
pub struct Layer {
    /// The render order for this layer
    pub render_order: Vec<RenderEntity>,

    /// The state of this layer
    pub state: LayerState,

    /// The stored states for this layer
    pub stored_states: Vec<LayerState>
}

impl Layer {
    ///
    /// Updates the transformation set for this layer
    ///
    pub fn update_transform(&mut self, active_transform: &canvas::Transform2D) {
        if &self.state.current_matrix != active_transform {
            // Update the current matrix
            self.state.current_matrix = *active_transform;

            // Add a 'set transform' to the rendering for this layer
            self.render_order.push(RenderEntity::SetTransform(*active_transform));
        }
    }

    ///
    /// Pushes a stored state for this layer
    ///
    pub fn push_state(&mut self) {
        self.stored_states.push(self.state.clone());
    }

    ///
    /// If this layer has any stored states, restores the most recent one
    ///
    pub fn pop_state(&mut self) {
        self.stored_states.pop()
            .map(|restored_state| self.state = restored_state);
    }
}
