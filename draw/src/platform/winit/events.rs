use super::traits::*;

use flo_canvas_events::*;

use flo_scene::*;
use flo_stream::*;

use once_cell::sync::{Lazy};
use winit::window::{WindowId};

use std::sync::*;
use serde::*;

static WINIT_EVENTS: Lazy<Publisher<WinitEvents>> = Lazy::new(|| Publisher::new(10));

///
/// Events supplied by the winit thread to allow external tracking of the actions
///
#[derive(Clone)]
pub enum WinitEvents {
    /// Indicates that a new winit window has been created
    CreatedWindow {
        /// The winit window ID
        window_id: WindowId,

        /// The scale factor that the window is created at
        scale: f64,

        /// The draw events for this window
        events: Arc<WeakPublisher<DrawEvent>>,

        // The winit platform window that was created
        platform_window: Arc<dyn PlatformWindow>,
    }
}

impl SceneMessage for WinitEvents {
    fn serializable() -> bool {
        false
    }
}

impl Serialize for WinitEvents {
    fn serialize<S>(&self, _: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer 
    {
        use serde::ser::*;
        Err(S::Error::custom("WinitEvents cannot be serialized"))
    }
}

impl<'a> Deserialize<'a> for WinitEvents {
    fn deserialize<D>(_: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'a> 
    {
        use serde::de::*;
        Err(D::Error::custom("WinitEvents cannot be serialized"))
    }
}

///
/// Retrieves the publisher for the winit events for the current process
///
pub fn winit_events() -> Publisher<WinitEvents> {
    (*WINIT_EVENTS).republish()
}
