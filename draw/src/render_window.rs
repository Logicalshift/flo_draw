use super::draw_event::*;
use super::glutin_thread::*;
use super::glutin_thread_event::*;

use flo_stream::*;
use flo_render::*;

///
/// Creates a window that can be rendered to by sending groups of render actions
///
pub fn create_render_window() -> (Publisher<Vec<RenderAction>>, Subscriber<DrawEvent>) {
    // Create the publisher to send the render actions to the stream
    let mut render_publisher    = Publisher::new(1);
    let mut event_publisher     = Publisher::new(1000);

    let event_subscriber        = event_publisher.subscribe();

    // Create a window that subscribes to the publisher
    let glutin_thread           = glutin_thread();
    glutin_thread.send_event(GlutinThreadEvent::CreateRenderWindow(render_publisher.subscribe(), event_publisher));

    // Publisher can now be used to render to the window
    (render_publisher, event_subscriber)
}
