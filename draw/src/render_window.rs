use super::glutin_thread::*;
use super::glutin_thread_event::*;

use flo_stream::*;
use flo_render::*;

///
/// Creates a window that can be rendered to by sending groups of render actions
///
pub fn create_window_render() -> Publisher<Vec<RenderAction>> {
    // Create the publisher to send the render actions to the stream
    let mut publisher   = Publisher::new(1);

    // Create a window that subscribes to the publisher
    let glutin_thread   = glutin_thread();
    glutin_thread.send_event(GlutinThreadEvent::CreateRenderOnlyWindow(publisher.subscribe()));

    // Publisher can now be used to render to the window
    publisher
}
