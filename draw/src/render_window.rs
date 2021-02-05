use super::draw_event::*;
use super::glutin_thread::*;
use super::window_properties::*;
use super::glutin_thread_event::*;

use flo_stream::*;
use flo_render::*;

use futures::prelude::*;

///
/// Creates a window that can be rendered to by sending groups of render actions
///
pub fn create_render_window<'a, TProperties: 'a+FloWindowProperties>(properties: TProperties) -> (Publisher<Vec<RenderAction>>, impl Clone+Send+Stream<Item=DrawEvent>) {
    // Create the publisher to send the render actions to the stream
    let window_properties       = WindowProperties::from(&properties);
    let mut render_publisher    = Publisher::new(1);
    let mut event_publisher     = Publisher::new(1000);

    let event_subscriber        = event_publisher.subscribe();

    // Create a window that subscribes to the publisher
    let glutin_thread           = glutin_thread();
    glutin_thread.send_event(GlutinThreadEvent::CreateRenderWindow(render_publisher.subscribe(), event_publisher, window_properties));

    // Publisher can now be used to render to the window
    (render_publisher, event_subscriber)
}
