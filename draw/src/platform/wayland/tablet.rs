use super::dispatch::*;
use super::event_queue::*;

use flo_scene::*;

use futures::prelude::*;

use std::collections::*;

use winit::event_loop::*;
use winit::raw_window_handle_05::{RawDisplayHandle, HasRawDisplayHandle};
use winit::window::{WindowId};

use wayland_client::{Connection, QueueHandle, Dispatch};
use wayland_client::backend::{Backend};
use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::wl_registry::{Event, WlRegistry};

///
/// State information for tracking wayland tablet events
///
pub struct WaylandTabletState {
    /// Used to dispatch futures in response to tablet events
    dispatcher: FloWaylandDispatcher,

    /// The window definitions, mapped from a wayland surface ID
    windows: HashMap<usize, WaylandTabletWindow>
}

///
/// Details about a window that we're tracking tablet events for
///
struct WaylandTabletWindow {
    /// The winit window ID
    window_id: WindowId,

    /// The scale factor for this window, used for calculating coordinates
    scale: f64,
}

impl FloWaylandState for WaylandTabletState {
    fn dispatcher(&mut self) -> Option<&mut FloWaylandDispatcher> {
        Some(&mut self.dispatcher)
    }
}

impl Dispatch<WlRegistry, GlobalListContents> for WaylandTabletState {
    fn event(_state: &mut Self, _registry: &WlRegistry, _event: Event, _data: &GlobalListContents, _conn: &Connection, _qh: &QueueHandle<Self>) {}
}

///
/// Runs the wayland tablet program, which sends tablet events to windows
///
pub fn wayland_tablet_program<TEvent>(input: InputStream<WaylandEventQueue<WaylandTabletState>>, context: SceneContext, event_loop: &EventLoop<TEvent>) -> impl 'static + Future<Output=()> {
    // Create a wayland backend from the event loop
    let display_ptr = event_loop.raw_display_handle();
    let proxy       = event_loop.create_proxy();

    let wayland_backend = if let RawDisplayHandle::Wayland(handle) = display_ptr {
        Some(unsafe { Backend::from_foreign_display(handle.display.cast()) })
    } else {
        None
    };

    async move {
        // Stop immediately if this isn't a wayland event loop
        let Some(wayland_backend) = wayland_backend else { return; };

        // Create the connection in guest mode
        let connection = Connection::from_backend(wayland_backend);

        // Initialise the queue
        let Ok((globals, mut event_queue)) = registry_queue_init::<WaylandTabletState>(&connection) else { return; };
        let queue_handle = event_queue.handle();

        // Set up the initial state
        let state = WaylandTabletState {
            dispatcher: FloWaylandDispatcher::new(),
            windows:    HashMap::new(),
        };

        // Run the queue
        wayland_event_queue_subprogram(input, context, event_queue, state).await;
    }
}
