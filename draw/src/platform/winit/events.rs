use flo_canvas_events::*;

use flo_stream::*;

use once_cell::sync::{Lazy};
use winit::window::{WindowId};

use std::sync::*;

static WINIT_EVENTS: Lazy<Publisher<WinitEvents>> = Lazy::new(|| Publisher::new(10));

///
/// Provides details about the winit window that was created
///
#[derive(Copy, Clone, Debug)]
pub enum WinitWindowHandle {
    /// A wayland window
    Wayland {
        /// The raw surface pointer to this window, cast to a usize (for comparison with wayland events from other sources)
        surface_ptr: usize,
    },

    /// Unknown type of winit window
    Other,
}

///
/// Events supplied by the winit thread to allow external tracking of the actions
///
#[derive(Clone)]
pub enum WinitEvents {
    /// Indicates that a new winit window has been created
    CreatedWindow {
        /// The winit window ID
        window_id: WindowId,

        /// Details about the window handle
        handle: WinitWindowHandle,

        /// The scale factor that the window is created at
        scale: f64,

        /// The draw events for this window
        events: Arc<WeakPublisher<DrawEvent>>,
    }
}

///
/// Retrieves the publisher for the winit events for the current process
///
pub fn winit_events() -> Publisher<WinitEvents> {
    (*WINIT_EVENTS).republish()
}
