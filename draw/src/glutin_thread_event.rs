use flo_stream::*;
use flo_render::*;

///
/// Event that can be sent to a glutin thread
///
#[derive(Clone)]
pub enum GlutinThreadEvent {
    /// Creates a window that will render the specified actions
    CreateRenderOnlyWindow(Subscriber<Vec<RenderAction>>)
}
