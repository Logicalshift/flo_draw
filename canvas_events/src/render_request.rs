use flo_render::*;
use flo_scene::*;

///
/// A request to a low-level render target
///
#[derive(Debug)]
pub enum RenderRequest {
    /// Performs the specified set of render actions immediately
    Render(Vec<RenderAction>)
}

impl SceneMessage for RenderRequest {
}
