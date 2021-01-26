use super::draw_event::*;

use flo_stream::*;
use flo_render::*;

use futures::future::{LocalBoxFuture};

///
/// Event that can be sent to a glutin thread
///
pub enum GlutinThreadEvent {
    /// Creates a window that will render the specified actions
    CreateRenderWindow(Subscriber<Vec<RenderAction>>, Publisher<DrawEvent>),

    /// Runs a future on the Glutin thread
    RunProcess(Box<dyn Send+FnOnce() -> LocalBoxFuture<'static, ()>>)
}
