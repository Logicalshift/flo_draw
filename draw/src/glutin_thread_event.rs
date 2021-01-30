use super::draw_event::*;

use flo_stream::*;
use flo_render::*;

use futures::future::{LocalBoxFuture};

use glutin::window::{WindowId};

///
/// Event that can be sent to a glutin thread
///
pub enum GlutinThreadEvent {
    /// Creates a window that will render the specified actions
    CreateRenderWindow(Subscriber<Vec<RenderAction>>, Publisher<DrawEvent>),

    /// Runs a future on the Glutin thread
    RunProcess(Box<dyn Send+FnOnce() -> LocalBoxFuture<'static, ()>>),

    /// Polls the future with the specified ID
    WakeFuture(u64),

    /// Stop sending events for the specified window
    StopSendingToWindow(WindowId),

    /// Tells the UI thread to stop when there are no more windows open
    StopWhenAllWindowsClosed
}
