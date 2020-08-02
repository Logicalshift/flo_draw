use super::stroke_settings::*;

use flo_canvas as canvas;
use flo_render as render;

use lyon::tessellation::{VertexBuffers};

///
/// Operation to use when drawing an item on a layer
///
#[derive(Clone, Copy)]
pub enum LayerOperation {
    /// Draw the vertex buffer
    Draw,

    /// Erase the vertex buffer
    Erase
}

///
/// Single rendering operation for a layer
///
pub enum RenderEntity {
    /// Render operation is missing
    Missing,

    /// Render operation is waiting to be tessellated (with a unique entity ID)
    Tessellating(LayerOperation, usize),

    /// Tessellation waiting to be sent to the renderer
    VertexBuffer(LayerOperation, VertexBuffers<render::Vertex2D, u16>),

    /// Render a vertex buffer
    DrawIndexed(LayerOperation, render::VertexBufferId, render::IndexBufferId, usize),

    /// Updates the transformation matrix for the layer
    SetTransform(canvas::Transform2D),

    /// Sets the blend mode to use for the following rendering
    SetBlendMode(render::BlendMode)
}

///
/// The current state of a layer
///
#[derive(Clone)]
pub struct LayerState {
    /// The current fill colour
    pub fill_color: render::Rgba8,

    /// The blend mode set for this layer
    pub blend_mode: canvas::BlendMode,

    /// The settings for the next brush stroke
    pub stroke_settings: StrokeSettings,

    /// Where the canvas's rendering should be rolled back to on the next 'restore' operation
    pub restore_point: Option<usize>,

    /// The current transformation matrix for this layer
    pub current_matrix: canvas::Transform2D
}

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
