use flo_canvas_events::*;
use objc2_foundation::{NSPoint};
use objc2_app_kit::{NSView, NSEvent};

///
/// Converts an NSEvent from a pointer event to a DrawEvent::Pointer event, with the specified action
///
pub fn draw_pointer_event_for_nsevent(view: &NSView, action: PointerAction, buttons: Vec<Button>, event: &NSEvent) -> DrawEvent {
    // Pointer ID
    let pointer_id      = event.pointingDeviceID();
    let pointer_id      = PointerId(pointer_id as _);

    // Grab various bits of the event
    let pos             = event.locationInWindow();
    let pressure        = event.pressure();
    let tilt            = event.tilt();
    let rotation        = event.rotation();

    // Convert to view coordinates
    let scale_factor    = view.window().map(|window| window.backingScaleFactor()).unwrap_or(1.0);
    let pos             = view.convertPoint_fromView(pos, None);
    let pos             = NSPoint { x: pos.x*scale_factor, y: pos.y*scale_factor };

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
