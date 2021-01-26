use super::draw_event::*;

use flo_stream::*;
use flo_render::*;

///
/// Event that can be sent to a glutin thread
///
pub enum GlutinThreadEvent {
    /// Creates a window that will render the specified actions
    CreateRenderWindow(Subscriber<Vec<RenderAction>>, Publisher<DrawEvent>)
}
