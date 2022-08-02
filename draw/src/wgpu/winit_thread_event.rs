use crate::events::*;
use crate::window_properties::*;

use flo_stream::*;
use flo_render::*;

use futures::future::{LocalBoxFuture};
use futures::stream::{BoxStream};
use futures::channel::oneshot;

use winit::window::{WindowId};

use std::fmt;
use std::fmt::*;

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

    /// Resolves a yield request by sending an empty message (used to yield to process events)
    Yield(oneshot::Sender<()>),

    /// Stop sending events for the specified window
    StopSendingToWindow(WindowId),

    /// Tells the UI thread to stop when there are no more windows open
    StopWhenAllWindowsClosed,
}

impl Debug for WinitThreadEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use self::WinitThreadEvent::*;

        match self {
            CreateRenderWindow(_, _, _)     => write!(f, "CreateRenderWindow(...)"),
            RunProcess(_)                   => write!(f, "RunProcess(...)"),
            WakeFuture(id)                  => write!(f, "WakeFuture({})", id),
            Yield(_)                        => write!(f, "Yield(...)"),
            StopSendingToWindow(id)         => write!(f, "StopSendingToWindow({:?})", id),
            StopWhenAllWindowsClosed        => write!(f, "StopWhenAllWindowsClosed"),
        }
    }
}