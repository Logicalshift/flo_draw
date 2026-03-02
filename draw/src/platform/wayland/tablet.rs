use super::dispatch::*;
use super::event_queue::*;

use flo_scene::*;
use flo_canvas_events as canvas_events;

use futures::prelude::*;

use std::collections::*;

use winit::event_loop::*;
use winit::raw_window_handle_05::{RawDisplayHandle, HasRawDisplayHandle, RawWindowHandle};
use winit::window::{WindowId};

use wayland_client::{Connection, QueueHandle, Dispatch, event_created_child, Proxy};
use wayland_client::backend::{Backend, ObjectId};
use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::wl_registry::{Event, WlRegistry};
use wayland_client::protocol::wl_seat::{self, WlSeat};
use wayland_protocols::wp::tablet::zv2::client::{
    *,
    zwp_tablet_v2::*,
    zwp_tablet_manager_v2::*,
    zwp_tablet_seat_v2::*,
    zwp_tablet_tool_v2::*,
    zwp_tablet_pad_v2::*,
    zwp_tablet_pad_group_v2::*,
    zwp_tablet_pad_ring_v2::*,
    zwp_tablet_pad_strip_v2::*,
};
use once_cell::sync::{Lazy};

/// The program ID where the wayland tablet program runs
pub static WAYLAND_TABLET_SUBPROGRAM: Lazy<SubProgramId> = Lazy::new(|| SubProgramId::called("flo_draw::wayland::tablet"));

///
/// State information for tracking wayland tablet events
///
pub struct WaylandTabletState {
    /// Used to dispatch futures in response to tablet events
    dispatcher: FloWaylandDispatcher,

    /// The window definitions, mapped from a wayland surface pointer
    window_for_surface: HashMap<usize, WaylandTabletWindow>,

    /// The tools that are used with the tablet
    tools: HashMap<ObjectId, TabletTool>,
}

///
/// Data stored with a tablet tool
///
struct TabletTool {
    /// Pointer ID assigned to this tool
    pointer_id: canvas_events::PointerId,
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

impl Dispatch<WlSeat, ()> for WaylandTabletState {
    fn event(_state: &mut Self, _proxy: &WlSeat, _event: wl_seat::Event, _data: &(), _conn: &Connection, _queue_handle: &QueueHandle<Self>) {
        // Used so we can bind the tablet manager later on
    }
}

impl Dispatch<ZwpTabletManagerV2, ()> for WaylandTabletState {
    fn event(_state: &mut Self, _proxy: &ZwpTabletManagerV2, _event: zwp_tablet_manager_v2::Event, _data: &(), _conn: &Connection, _queue_handle: &QueueHandle<Self>) {
        // Used to bind the tablet
    }
}

impl Dispatch<ZwpTabletSeatV2, ()> for WaylandTabletState {
    fn event(state: &mut Self, _proxy: &ZwpTabletSeatV2, event: zwp_tablet_seat_v2::Event, _data: &(), _conn: &Connection, _queue_handle: &QueueHandle<Self>) {
        match event {
            zwp_tablet_seat_v2::Event::ToolAdded { id } => {
                let tool_object_id = id.id();

                // Assign a pointer ID for this tool
                let pointer_id = state.tools.len() + 1;
                let pointer_id = canvas_events::PointerId(pointer_id as _);

                // Create a structure for tracking the state of this tool
                let tool = TabletTool {
                    pointer_id
                };

                state.tools.insert(tool_object_id, tool);
            },

            zwp_tablet_seat_v2::Event::TabletAdded { .. }   => {},
            zwp_tablet_seat_v2::Event::PadAdded { .. }      => {},
            _                                               => {},
        }
    }

    event_created_child!(WaylandTabletState, ZwpTabletSeatV2, [
        EVT_TABLET_ADDED_OPCODE => (ZwpTabletV2, ()),
        EVT_TOOL_ADDED_OPCODE   => (ZwpTabletToolV2, ()),
        EVT_PAD_ADDED_OPCODE    => (ZwpTabletPadV2, ()),
    ]);
}

impl Dispatch<ZwpTabletToolV2, ()> for WaylandTabletState {
    fn event(_state: &mut Self, _proxy: &ZwpTabletToolV2, _event: zwp_tablet_tool_v2::Event, _data: &(), _conn: &Connection, _queue_handle: &QueueHandle<Self>) {
        // TODO: tablet events
        println!("{:?}", _event);
    }
}

impl Dispatch<ZwpTabletV2, ()> for WaylandTabletState {
    fn event(_state: &mut Self, _proxy: &ZwpTabletV2, _event: zwp_tablet_v2::Event, _data: &(), _conn: &Connection, _queue_handle: &QueueHandle<Self>) {
        // Only the tablet tool events are used at the moment
    }
}

impl Dispatch<ZwpTabletPadV2, ()> for WaylandTabletState {
    fn event(_state: &mut Self, _proxy: &ZwpTabletPadV2, _event: zwp_tablet_pad_v2::Event, _data: &(), _conn: &Connection, _queue_handle: &QueueHandle<Self>) {
        // Not actually used, but panics without it
    }

    event_created_child!(WaylandTabletState, ZwpTabletPadV2, [
        EVT_GROUP_OPCODE => (ZwpTabletPadGroupV2, ()),
    ]);
}

impl Dispatch<ZwpTabletPadGroupV2, ()> for WaylandTabletState {
    fn event(_state: &mut Self, _proxy: &ZwpTabletPadGroupV2, _event: zwp_tablet_pad_group_v2::Event, _data: &(), _conn: &Connection, _queue_handle: &QueueHandle<Self>) {
        // Not used, but required for event handling
    }

    event_created_child!(WaylandTabletState, ZwpTabletPadGroupV2, [
        EVT_RING_OPCODE     => (ZwpTabletPadRingV2,  ()),
        EVT_STRIP_OPCODE    => (ZwpTabletPadStripV2, ()),
    ]);
}

impl Dispatch<ZwpTabletPadRingV2, ()> for WaylandTabletState {
    fn event(_state: &mut Self, _proxy: &ZwpTabletPadRingV2, _event: zwp_tablet_pad_ring_v2::Event, _data: &(), _conn: &Connection, _queue_handle: &QueueHandle<Self>) {
        // Only the tablet tool events are used at the moment
    }
}

impl Dispatch<ZwpTabletPadStripV2, ()> for WaylandTabletState {
    fn event(_state: &mut Self, _proxy: &ZwpTabletPadStripV2, _event: zwp_tablet_pad_strip_v2::Event, _data: &(), _conn: &Connection, _queue_handle: &QueueHandle<Self>) {
        // Only the tablet tool events are used at the moment
    }
}

///
/// Runs the wayland tablet program, which sends tablet events to windows
///
pub fn wayland_tablet_program(input: InputStream<WaylandEventQueue<WaylandTabletState>>, context: SceneContext, display_handle: OwnedDisplayHandle) -> impl 'static + Future<Output=()> {
    // Create a wayland backend from the event loop
    let display_ptr = display_handle.raw_display_handle();

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
        let Ok((globals, event_queue)) = registry_queue_init::<WaylandTabletState>(&connection) else { return; };
        let queue_handle = event_queue.handle();

        // Set up the initial state
        let state = WaylandTabletState {
            dispatcher:         FloWaylandDispatcher::new(),
            window_for_surface: HashMap::new(),
            tools:              HashMap::new(),
        };

        // Bind the tablet manager (we'll leave with no tablet program if the manager fails to bind)
        let Ok(tablet_manager) = globals.bind::<ZwpTabletManagerV2, _, _>(&queue_handle, 1..=1, ()) else { return; };

        // Create the tablet seat to bind the events
        let Ok(seat)     = globals.bind::<WlSeat, _, _>(&queue_handle, 1..=9, ()) else { return; };
        let _tablet_seat = tablet_manager.get_tablet_seat(&seat, &queue_handle, ());

        // Run the queue
        wayland_event_queue_subprogram(input, context, event_queue, state).await;
    }
}

///
/// Calls the tablet program for the scene to register a window
///
pub async fn add_wayland_tablet_window(context: &SceneContext, raw_handle: RawWindowHandle, window_id: WindowId, initial_scale: f64) {
    if let RawWindowHandle::Wayland(window_handle) = raw_handle {
        // Cast the surface ptr to a usize to allow us to look it up later on
        let surface_ptr = window_handle.surface as usize;

        context.send_message(WaylandEventQueue::<WaylandTabletState>::UpdateState(Box::new(move |tablet_state| {
            // Create a new window
            let new_window = WaylandTabletWindow {
                window_id:  window_id,
                scale:      initial_scale,
            };

            // Add it to the state
            tablet_state.window_for_surface.insert(surface_ptr, new_window);
        }))).await.ok();
    }
}

///
/// Calls the tablet program for the scene to register a window
///
pub async fn set_tablet_window_scale(context: &SceneContext, raw_handle: RawWindowHandle, new_scale: f64) {
    if let RawWindowHandle::Wayland(window_handle) = raw_handle {
        // Cast the surface ptr to a usize to allow us to look it up later on
        let surface_ptr = window_handle.surface as usize;

        context.send_message(WaylandEventQueue::<WaylandTabletState>::UpdateState(Box::new(move |tablet_state| {
            // Change the scale of the existing window
            let Some(window) = tablet_state.window_for_surface.get_mut(&surface_ptr) else { return; };
            window.scale = new_scale;
        }))).await.ok();
    }
}
