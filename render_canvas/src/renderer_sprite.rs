use super::renderer_layer::*;


///
/// Definition of a canvas sprite
///
pub struct Sprite {
    /// The render order for this layer
    pub render_order: Vec<RenderEntity>,

    /// The state of this layer
    pub state: LayerState,

    /// The stored states for this sprite
    pub stored_states: Vec<LayerState>
}
