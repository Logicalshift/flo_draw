use flo_render::*;

///
/// A request to a low-level render target
///
pub enum RenderRequest {
    /// Performs the specified set of render actions immediately
    Render(Vec<RenderAction>)
}
