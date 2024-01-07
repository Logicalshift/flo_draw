use crate::draw_scene::*;
use crate::events::*;
use crate::window_properties;
use crate::window_properties::*;

use flo_scene::*;
use flo_scene::programs::*;
use flo_stream::*;
use flo_render::*;
use flo_binding::*;

use futures::prelude::*;
use futures::stream;

use std::sync::*;
use futures::channel::mpsc;

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
/// Sends the events for changing the properties in a set of WindowProperties
///
pub (crate) async fn send_window_properties<TRequest>(context: &SceneContext, window_properties: WindowProperties, target: SubProgramId) -> Result<(), ConnectionError>
where
    TRequest: 'static + Send + SceneMessage + From<EventWindowRequest>,
{
    context.send_message(SceneControl::start_program(SubProgramId::new(),
        move |_: InputStream<()>, context| {
            let window_properties = window_properties.clone();

            async move {
                // Follow the properties
                let title           = follow(window_properties.title);
                let fullscreen      = follow(window_properties.fullscreen);
                let has_decorations = follow(window_properties.has_decorations);
                let mouse_pointer   = follow(window_properties.mouse_pointer);

                // Each one generates an event when it changes
                let title           = title.map(|new_title| EventWindowRequest::SetTitle(new_title));
                let fullscreen      = fullscreen.map(|fullscreen| EventWindowRequest::SetFullScreen(fullscreen));
                let has_decorations = has_decorations.map(|has_decorations| EventWindowRequest::SetHasDecorations(has_decorations));
                let mouse_pointer   = mouse_pointer.map(|mouse_pointer| EventWindowRequest::SetMousePointer(mouse_pointer));

                let mut requests    = stream::select_all(vec![
                    title.boxed(),
                    fullscreen.boxed(),
                    has_decorations.boxed(),
                    mouse_pointer.boxed(),
                ]);

                // Pass the requests on to the underlying window
                let channel = context.send::<TRequest>(target);
                if let Ok(mut channel) = channel {
                    while let Some(request) = requests.next().await {
                        channel.send(request.into()).await.ok();
                    }
                }
            }
        }, 0));

    Ok(())
}

///
/// Creates a window that renders a stream of actions
///
pub fn create_render_window_from_stream<'a, RenderStream, TProperties>(render_stream: RenderStream, properties: TProperties) -> impl Send + Stream<Item=DrawEvent>
where
    RenderStream:   'static + Send + Stream<Item=Vec<RenderAction>>,
    TProperties:    'a + FloWindowProperties,
{
    let properties              = WindowProperties::from(&properties);

    // Create a new render window entity
    let render_window_program   = SubProgramId::new();
    let scene_context           = flo_draw_scene_context();

    let render_channel          = create_render_window_sub_program(&scene_context, render_window_program, properties.size().get()).unwrap();

    // Use a channel to get the events out of the program
    let (send_events, recv_events)  = mpsc::channel(20);
    let event_relay_program         = SubProgramId::new();
    scene_context.add_subprogram(event_relay_program,
        move |mut draw_events: InputStream<DrawEvent>, _| async move {
            while let Some(event) = draw_events.next().await {
                match send_events.send(event).await {
                    Ok(())  => { },
                    Err(_)  => { break; }
                };
            }
        },
        20);

    // Pass events from the render stream onto the window using another entity (potentially this could be a background task for the render window entity?)
    let process_program         = SubProgramId::new();
    scene_context.add_subprogram(process_program, move |_: InputStream<()>, context| {
        async move {
            let mut render_stream   = render_stream.boxed();
            let mut render_channel  = render_channel;

            send_window_properties(&context, properties, render_window_program).await.ok();

            // Request event actions from the renderer
            render_channel.send(RenderWindowRequest::SendEvents(event_relay_program)).await.ok();

            // Main loop passes on the render actions (we don't process messages directed at this entity)
            while let Some(render_actions) = render_stream.next().await {
                let maybe_err = render_channel.send(RenderWindowRequest::Render(RenderRequest::Render(render_actions))).await;

                if maybe_err.is_err() {
                    // Stop if the request doesn't go through
                    break;
                }
            }
        }
    }, 0);

    // The events stream is the result
    recv_events
}
