use crate::draw_scene::*;
use crate::events::*;
use crate::window_properties::*;

use flo_scene::*;
use flo_stream::*;
use flo_render::*;

use futures::prelude::*;

///
/// Creates a window that can be rendered to by sending groups of render actions
///
pub fn create_render_window<'a, TProperties>(properties: TProperties) -> (Publisher<Vec<RenderAction>>, impl Send + Stream<Item=DrawEvent>)
where
    TProperties: 'a+FloWindowProperties,
{
    // Create the publisher to send the render actions to the stream
    let mut render_publisher    = Publisher::new(1);
    let event_subscriber        = create_render_window_from_stream(render_publisher.subscribe(), properties);

    // Publisher can now be used to render to the window
    (render_publisher, event_subscriber)
}

///
/// Creates a window that renders a stream of actions
///
pub fn create_render_window_from_stream<'a, RenderStream, TProperties>(render_stream: RenderStream, properties: TProperties) -> impl Send + Stream<Item=DrawEvent>
where
    RenderStream:   'static + Send + Stream<Item=Vec<RenderAction>>,
    TProperties:    'a + FloWindowProperties,
{
    // TODO: add a way to follow the window properties

    // Create a new render window entity
    let render_window_entity    = EntityId::new();
    let scene_context           = flo_draw_scene_context();

    let render_channel          = create_render_window_entity(&scene_context, render_window_entity).unwrap();

    // The events send to a channel
    let (events_channel, events_stream) = SimpleEntityChannel::new(render_window_entity, 5);

    // Pass events from the render stream onto the window using another entity (potentially this could be a background task for the render window entity?)
    let process_entity = EntityId::new();
    scene_context.create_entity::<(), (), _, _>(process_entity, move |_, _| {
        async move {
            let mut render_stream   = render_stream.boxed();
            let mut render_channel  = render_channel;

            // Request event actions from the renderer
            render_channel.send(RenderWindowRequest::SendEvents(events_channel.boxed())).await.ok();

            // Main loop passes on the render actions (we don't process messages directed at this entity)
            while let Some(render_actions) = render_stream.next().await {
                let maybe_err = render_channel.send_without_waiting(RenderWindowRequest::Render(RenderRequest::Render(render_actions))).await;

                if maybe_err.is_err() {
                    // Stop if the request doesn't go through
                    break;
                }
            }
        }
    }).unwrap();

    // We don't process messages in our background entity, so seal it off
    scene_context.seal_entity(process_entity).unwrap();

    // The events stream is the result
    events_stream.map(|msg| {
        let (evt, response) = msg.take();
        response.send(()).ok();

        evt
    })
}
