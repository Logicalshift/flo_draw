use super::render_request::*;
use super::drawing_request::*;
use super::draw_event_request::*;

use flo_scene::*;

///
/// Messages that can be sent to a flo_draw window that processes 2D graphics instructions
///
pub enum DrawingWindowRequest {
    /// Carry out a drawing request
    Draw(DrawingRequest),

    /// Adds a channel that events generated for this window is relayed to
    SendEvents(BoxedEntityChannel<'static, DrawEventRequest, ()>),
}

///
/// Messages that can be sent to a flo_draw window that processes low-level 2D graphics instructions
///
pub enum RenderWindowRequest {
    /// Carry out a render request
    Render(RenderRequest),

    /// Adds a channel that events generated for this window is relayed to
    SendEvents(BoxedEntityChannel<'static, DrawEventRequest, ()>),
}
