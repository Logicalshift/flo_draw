use flo_canvas_events::*;

use flo_stream::*;

use once_cell::sync::{Lazy};
use winit::window::{WindowId};
use winit::raw_window_handle_05::{RawWindowHandle};

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

    UiKit,
    AppKit,
    Orbital,
    Xlib,
    Xcb,
    Drm,
    Gbm,
    Win32,
    WinRt,
    Web,
    AndroidNdk,
    Haiku,

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

impl WinitWindowHandle {
    ///
    /// Creates a winit window handle from a raw winit window handle
    ///
    pub fn from_raw_window_handle(handle: &RawWindowHandle) -> Self {
        match handle {
            RawWindowHandle::Wayland(wayland_window_handle)         => Self::Wayland { surface_ptr: wayland_window_handle.surface as usize },

            RawWindowHandle::UiKit(_ui_kit_window_handle)           => Self::UiKit,
            RawWindowHandle::AppKit(_app_kit_window_handle)         => Self::AppKit,
            RawWindowHandle::Orbital(_orbital_window_handle)        => Self::Orbital,
            RawWindowHandle::Xlib(_xlib_window_handle)              => Self::Xlib,
            RawWindowHandle::Xcb(_xcb_window_handle)                => Self::Xcb,
            RawWindowHandle::Drm(_drm_window_handle)                => Self::Drm,
            RawWindowHandle::Gbm(_gbm_window_handle)                => Self::Gbm,
            RawWindowHandle::Win32(_win32_window_handle)            => Self::Win32,
            RawWindowHandle::WinRt(_win_rt_window_handle)           => Self::WinRt,
            RawWindowHandle::Web(_web_window_handle)                => Self::Web,
            RawWindowHandle::AndroidNdk(_android_ndk_window_handle) => Self::AndroidNdk,
            RawWindowHandle::Haiku(_haiku_window_handle)            => Self::Haiku,

            _ => Self::Other,
        }

    }
}

///
/// Retrieves the publisher for the winit events for the current process
///
pub fn winit_events() -> Publisher<WinitEvents> {
    (*WINIT_EVENTS).republish()
}
