use super::render_request::*;
use super::drawing_request::*;
use super::draw_window_request::*;

use crate::glutin_thread::*;
use crate::glutin_thread_event::*;
use crate::window_properties::*;

use futures::prelude::*;
use futures::channel::mpsc;

use flo_scene::*;
use flo_stream::*;

use std::sync::*;

///
/// Creates a render window in a scene with the specified entity ID
///
pub fn create_render_window(context: &Arc<SceneContext>, entity_id: EntityId) -> Result<SimpleEntityChannel<RenderWindowRequest, ()>, CreateEntityError> {
    // This window can accept a couple of converted messages
    context.convert_message::<RenderRequest, RenderWindowRequest>()?;
    context.convert_message::<EventWindowRequest, RenderWindowRequest>()?;

    // Create the window in context
    context.create_entity(entity_id, |context, render_window_requests| async move {
        // Create the publisher to send the render actions to the stream
        let window_properties   = WindowProperties::from(&());
        let mut event_publisher = Publisher::new(1000);

        // Create a stream for publishing render requests
        let (render_sender, render_receiver) = mpsc::channel(5);

        // Create a window that subscribes to the publisher
        let glutin_thread   = glutin_thread();
        glutin_thread.send_event(GlutinThreadEvent::CreateRenderWindow(render_receiver.boxed(), event_publisher.republish(), window_properties.into()));

        // Run the main event loop
        let mut render_window_requests  = render_window_requests;
        let mut render_sender           = render_sender;

        while let Some(request) = render_window_requests.next().await {
            let request: Message::<RenderWindowRequest, ()> = request;

            // Take the request so we can send the contained data directly
            let (request, response) = request.take();

            match request {
                RenderWindowRequest::Render(RenderRequest::Render(render)) => {
                    // Just pass render requests on to the render window
                    if render_sender.send(render).await.is_err() {
                        // This entity is finished if the window finishes
                        break;
                    }
                }

                RenderWindowRequest::SendEvents(channel_target) => {
                    let mut subscriber = event_publisher.subscribe();

                    context.run_in_background(async move {
                        let mut channel_target = channel_target;

                        // Pass on events to everything that's listening, until the channel starts generating errors
                        while let Some(event) = subscriber.next().await {
                            let result = channel_target.send_without_waiting(event).await;

                            if result.is_err() {
                                break;
                            }
                        }
                    }).ok();
                }
            }

            response.send(()).ok();
        }
    })
}
