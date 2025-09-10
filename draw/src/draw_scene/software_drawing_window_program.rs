use crate::software::*;
use crate::window_properties::*;

use futures::prelude::*;
use futures::channel::mpsc;

use flo_scene::*;
use flo_scene::programs::*;
use flo_stream::*;
use flo_binding::*;
use flo_canvas as canvas;
use flo_canvas::scenery::*;
use flo_canvas_events::*;

use std::sync::*;

///
/// Combines rendering and event messages into one enum
///
#[derive(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
enum DrawingOrEvent {
    Drawing(Vec<DrawingWindowRequest>),
    Event(Vec<DrawEventRequest>),
}

///
/// Creates a drawing window in a scene with the specified entity ID
///
pub fn create_software_draw_window_program(scene: &Arc<Scene>, program_id: SubProgramId, initial_size: (u64, u64)) -> Result<(), ConnectionError> {
    // Create the window in context
    scene.add_subprogram(program_id, move |drawing_window_requests, context| {
        // Window properties are used to relay the various properties to the winit thread
        // TODO: would make more sense just to directly relay these as messages
        let title               = bind("flo_draw".to_string());
        let fullscreen          = bind(false);
        let has_decorations     = bind(true);
        let mouse_pointer       = bind(MousePointer::SystemDefault);
        let requested_size      = bind(initial_size);
        let viewport_bounds     = bind(ViewportBounds::All);
        let window_properties   = WindowProperties::default()
            .with_title(title.clone())
            .with_fullscreen(fullscreen.clone())
            .with_has_decorations(has_decorations.clone())
            .with_mouse_pointer(mouse_pointer.clone())
            .with_requested_size(requested_size.clone())
            .with_viewport_bounds(viewport_bounds.clone());

        // The canvas transform is used to convert window coordinates to canvas coordinates
        let mut canvas_transform = canvas::Transform2D::identity();

        // The event publisher is used to receive events from the window
        let mut event_publisher = Publisher::new(1000);
        let drawing_events      = event_publisher.subscribe();

        // The event subscribers are the things that are subscribed to this program. We always create an initial subscriber because we assume something will start listening and we want to send all the events to that object
        let mut event_subscribers   = vec![];
        let mut initial_events      = vec![];

        // Create a stream for publishing render requests
        let (drawing_sender, drawing_receiver) = mpsc::channel(5);

        // Create a window that subscribes to the publisher (we do this outside of the main 'async' loop so this has happened on return)
        // If the window is not created immediately, there may be a race condition if `StopWhenAllWindowsClosed` is sent 
        let winit_thread = winit_thread();
        winit_thread.send_event(WinitThreadEvent::CreateDrawingWindow(drawing_receiver.boxed(), event_publisher.republish(), window_properties.into()));

        // Map the requests to DrawingOrEvent objects
        let drawing_window_requests = drawing_window_requests.ready_chunks(100).map(|requests| DrawingOrEvent::Drawing(requests));
        let drawing_events          = drawing_events.ready_chunks(100).map(|requests| DrawingOrEvent::Event(requests));

        let drawing_window_requests = stream::select(drawing_window_requests, drawing_events);

        async move {
            // Run the main event loop
            let mut drawing_window_requests = drawing_window_requests;
            let mut drawing_sender          = drawing_sender;

            while let Some(drawing_or_event) = drawing_window_requests.next().await {
                match drawing_or_event {
                    DrawingOrEvent::Drawing(drawing_requests) => {
                        // Process the requests from the window
                        for request in drawing_requests.into_iter() {
                            match request {
                                DrawingWindowRequest::Draw(DrawingRequest::Draw(drawing)) => {
                                    if drawing_sender.send(drawing).await.is_err() {
                                        // This entity is finished if the window finishes
                                        break;
                                    }
                                }

                                DrawingWindowRequest::Redraw => {
                                    // Trigger a redraw by sending an empty request
                                    if drawing_sender.send(Arc::new(vec![])).await.is_err() {
                                        // Stop when we can't send events any more
                                        break;
                                    }
                                }

                                DrawingWindowRequest::SendEvents(channel_target) => {
                                    if let Ok(mut target) = context.send(channel_target) {
                                        // Send the initial events if there are any (ie, any events that arrived before we had any subscribers)
                                        for evt in initial_events.drain(..) {
                                            target.send(evt).await.ok();
                                        }

                                        // Add to subscribers
                                        event_subscribers.push(target);
                                    }
                                }

                                DrawingWindowRequest::CloseWindow => {
                                    // The window will close its publisher in response to the events stream being closed
                                    drawing_sender.close().await.ok();

                                    // Shut down the event publisher
                                    use std::mem;
                                    let when_closed = event_publisher.when_closed();
                                    mem::drop(event_publisher);

                                    // Finally, wait for the publisher to finish up, and stop this program
                                    when_closed.await;
                                    return;
                                }

                                DrawingWindowRequest::SetTitle(new_title)                => { title.set(new_title); },
                                DrawingWindowRequest::SetFullScreen(new_fullscreen)      => { fullscreen.set(new_fullscreen); },
                                DrawingWindowRequest::SetHasDecorations(new_decorations) => { has_decorations.set(new_decorations); },
                                DrawingWindowRequest::SetMousePointer(new_mouse_pointer) => { mouse_pointer.set(new_mouse_pointer); },
                                DrawingWindowRequest::SetViewportBounds(new_bounds)      => { viewport_bounds.set(new_bounds); }
                            }
                        }
                    }

                    DrawingOrEvent::Event(drawing_events) => {
                        // Process the events
                        let mut perform_redraw  = false;
                        let mut stop            = false;
                        let mut drawing_events  = drawing_events;

                        for event in drawing_events.iter_mut() {
                            match event {
                                DrawEvent::Redraw       |
                                DrawEvent::Resize(_, _) |
                                DrawEvent::Scale(_)     => {
                                    perform_redraw = true;
                                }

                                DrawEvent::Closed => {
                                    stop = true;
                                }

                                DrawEvent::CanvasTransform(new_transform) => {
                                    canvas_transform = *new_transform;
                                }

                                DrawEvent::Pointer(_, _, state) => {
                                    // Update pointer events with an accurate location in canvas
                                    let location_in_canvas = canvas_transform.transform_point(state.location_in_window.0 as _, state.location_in_window.1 as _);

                                    state.location_in_canvas = Some((location_in_canvas.0 as _, location_in_canvas.1 as _));
                                }

                                // Ignore other events
                                _ => { }
                            }
                        }

                        if perform_redraw {
                            // Send an empty drawing request to force a redraw
                            if drawing_sender.send(Arc::new(vec![])).await.is_err() {
                                stop = true;
                            }
                        }


                        if event_subscribers.is_empty() {
                            // If there are no subscribers, buffer the event until there are some (up to 1000 events, presumably we're in a fairly stuck situation if we get more than that)
                            if initial_events.len() < 1000 {
                                // (If > 1000 events, we stop receiving to try to protect the rest of the program)
                                initial_events.extend(drawing_events);
                            }
                        } else {
                            // Send the event to the subscribers
                            let mut finished = vec![];

                            for idx in 0..event_subscribers.len() {
                                let subscriber = &mut event_subscribers[idx];

                                // Send all the events to this subscriber
                                for evt in drawing_events.iter() {
                                    if subscriber.send(evt.clone()).await.is_err() {
                                        // If the subscriber refuses an event, mark it as finished
                                        finished.push(idx);
                                    }
                                }
                            }

                            // Clear out any finished subscribers
                            for finished_idx in finished.into_iter().rev() {
                                event_subscribers.remove(finished_idx);
                            }
                        }

                        // Stop immediately if the window is closed
                        if stop {
                            break;
                        }
                    }
                }
            }
        }
    }, 20);

    Ok(())
}
