use flo_render as render;
use flo_canvas as canvas;

///
/// Ued to indicate the state of a gradient: these are loaded as 1-dimensional textures when they are used
///
#[derive(Clone)]
pub enum RenderGradient {
    Defined(Vec<canvas::GradientOp>),
    Ready(render::TextureId, Vec<canvas::GradientOp>)
}
