use super::layer_state::*;
use super::layer_bounds::*;
use super::render_entity::*;

use flo_canvas as canvas;

///
/// Definition of a layer in the canvas
///
pub struct Layer {
    /// The render order for this layer
    pub render_order: Vec<RenderEntity>,

    /// The bounds of the entities rendered to this layer
    pub bounds: LayerBounds,

    /// The state of this layer
    pub state: LayerState,

    /// True if this layer should be drawn on a fresh framebuffer (eg: due to blend mode of the layer or items in the layer)
    pub commit_before_rendering: bool,

    /// True if this layer should be drawn to the render buffer immediately after rendering (eg: due to blend mode)
    pub commit_after_rendering: bool,

    /// The blend mode to use when committing this layer (if committing after rendering)
    pub blend_mode: canvas::BlendMode,

    /// The alpha blend value to use for this layer (if committing after rendering)
    pub alpha: f64,

    /// The stored states for this layer
    pub stored_states: Vec<LayerState>
}

impl Layer {
    ///
    /// Updates the transformation set for this layer
    ///
    pub fn update_transform(&mut self, active_transform: &canvas::Transform2D) {
        if &self.state.current_matrix != active_transform && !self.state.is_sprite {
            // Update the current matrix
            self.state.current_matrix   = *active_transform;

            // Work out the scale factor from the matrix (skewed matrices won't produce accurate values here)
            let canvas::Transform2D([[_a, _b, _], [d, e, _], [_, _, _]]) = active_transform;
            // let scale_x              = a*a + b*b;
            let scale_y                 = d*d + e*e;

            self.state.scale_factor     = scale_y.sqrt();

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
