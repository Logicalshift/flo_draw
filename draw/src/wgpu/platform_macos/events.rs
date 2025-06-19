use flo_canvas_events::*;
use objc2_app_kit::{NSEvent};

///
/// Converts an NSEvent from a pointer event to a DrawEvent::Pointer event, with the specified action
///
pub fn draw_pointer_event_for_nsevent(action: PointerAction, buttons: Vec<Button>, event: &NSEvent) -> DrawEvent {
    // Pointer ID
    let pointer_id      = unsafe { event.pointingDeviceID() };
    let pointer_id      = PointerId(pointer_id as _);

    // Grab various bits of the event
    let pos             = unsafe { event.locationInWindow() };
    let pressure        = unsafe { event.pressure() };
    let tilt            = unsafe { event.tilt() };
    let rotation        = unsafe { event.rotation() };

    // Convert to a pointer state
    let pointer_state   = PointerState {
        location_in_window: (pos.x, pos.y),
        location_in_canvas: None,
        buttons:            buttons,
        pressure:           Some(pressure as _),
        tilt:               Some((tilt.x, tilt.y)),
        rotation:           Some(rotation as _),
        flow_rate:          None,
    };

    DrawEvent::Pointer(action, pointer_id, pointer_state)
}
