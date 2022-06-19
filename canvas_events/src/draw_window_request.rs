use super::render_request::*;
use super::drawing_request::*;
use super::draw_event_request::*;

use flo_scene::*;

///
/// Messages that can be sent to a flo_draw window that can generate events
///
pub enum EventWindowRequest {
    /// Adds a channel that events generated for this window is relayed to
    SendEvents(BoxedEntityChannel<'static, DrawEventRequest, ()>),
}


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

impl From<RenderRequest> for RenderWindowRequest {
    fn from(req: RenderRequest) -> RenderWindowRequest {
        RenderWindowRequest::Render(req)
    }
}

impl From<DrawingRequest> for DrawingWindowRequest {
    fn from(req: DrawingRequest) -> DrawingWindowRequest {
        DrawingWindowRequest::Draw(req)
    }
}

impl From<EventWindowRequest> for RenderWindowRequest {
    fn from(req: EventWindowRequest) -> RenderWindowRequest {
        match req {
            EventWindowRequest::SendEvents(events) => RenderWindowRequest::SendEvents(events)
        }
    }
}

impl From<EventWindowRequest> for DrawingWindowRequest {
    fn from(req: EventWindowRequest) -> DrawingWindowRequest {
        match req {
            EventWindowRequest::SendEvents(events) => DrawingWindowRequest::SendEvents(events)
        }
    }
}
