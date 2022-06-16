use super::render_request::*;
use super::drawing_request::*;
use super::draw_window_request::*;
use super::draw_event_request::*;

use crate::glutin_thread::*;
use crate::glutin_thread_event::*;
use crate::window_properties::*;

use futures::prelude::*;
use futures::stream;
use futures::channel::mpsc;

use flo_scene::*;
use flo_stream::*;
use flo_render::*;
use flo_render_canvas::*;

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

///
/// Creates a drawing window that sends render requests to the specified target
///
pub fn create_drawing_window(context: &Arc<SceneContext>, entity_id: EntityId, render_target: impl 'static + EntityChannel<Message=RenderWindowRequest, Response=()>) -> Result<SimpleEntityChannel<DrawingWindowRequest, ()>, CreateEntityError> {
    // This window can accept a couple of converted messages
    context.convert_message::<DrawingRequest, DrawingWindowRequest>()?;
    context.convert_message::<EventWindowRequest, DrawingWindowRequest>()?;

    // Create the window in context
    context.create_entity(entity_id, move |context, drawing_window_requests| async move {
        let mut render_target       = render_target;

        // We relay events via our own event publisher
        // let mut event_publisher = Publisher::new(1000);

        // Set up the renderer and window state
        let renderer                = CanvasRenderer::new();
        //let mut window_transform    = None;
        let mut scale               = 1.0;
        let mut width               = 1.0;
        let mut height              = 1.0;

        // Request the events from the render target
        let (channel, events_receiver)  = SimpleEntityChannel::new(entity_id, 1000);
        render_target.send(RenderWindowRequest::SendEvents(channel.boxed())).await.ok();

        // Chunk the requests we receive
        let drawing_window_requests     = drawing_window_requests.chunks(100);
        let events_receiver             = events_receiver.chunks(100);

        // Combine the two streams (we prioritise events from the window to avoid spending time rendering with out-of-date state)
        enum DrawingOrEvent {
            Drawing(Vec<Message<DrawingWindowRequest, ()>>),
            Event(Vec<Message<DrawEventRequest, ()>>),
        }
        let drawing_window_requests     = drawing_window_requests.map(|evt| DrawingOrEvent::Drawing(evt));
        let events_receiver             = events_receiver.map(|evt| DrawingOrEvent::Event(evt));
        let messages                    = stream::select_with_strategy(drawing_window_requests, events_receiver, |_: &mut ()| stream::PollNext::Right);

        // Run the main event loop
        let mut messages = messages;
        while let Some(message) = messages.next().await {

        }
    })
}
