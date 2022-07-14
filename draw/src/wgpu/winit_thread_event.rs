use crate::events::*;
use crate::window_properties::*;

use flo_stream::*;
use flo_render::*;

use futures::future::{LocalBoxFuture};
use futures::stream::{BoxStream};

use winit::window::{WindowId};

///
/// Event that can be sent to a winit thread
///
pub enum WinitThreadEvent {
    /// Creates a window that will render the specified actions
    CreateRenderWindow(BoxStream<'static, Vec<RenderAction>>, Publisher<DrawEvent>, WindowProperties),

    /// Runs a future on the winit thread
    RunProcess(Box<dyn Send+FnOnce() -> LocalBoxFuture<'static, ()>>),

    /// Polls the future with the specified ID
    WakeFuture(u64),

    /// Stop sending events for the specified window
    StopSendingToWindow(WindowId),

    /// Tells the UI thread to stop when there are no more windows open
    StopWhenAllWindowsClosed,
}
