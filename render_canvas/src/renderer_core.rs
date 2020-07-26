use super::renderer_layer::*;

///
/// Parts of the renderer that are shared with the workers
///
pub struct RenderCore {
    /// The definition for the layers
    pub layers: Vec<Layer>
}
