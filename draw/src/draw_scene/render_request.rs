use flo_render::*;

use std::sync::*;

///
/// A request to a low-level render target
///
pub enum RenderRequest {
    /// Performs the specified set of render actions immediately
    Render(Arc<Vec<RenderAction>>)
}
