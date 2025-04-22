use crate::software::*;
use crate::window_properties::*;

use futures::prelude::*;
use futures::channel::mpsc;
use once_cell::sync::{Lazy};

use flo_scene::*;
use flo_scene::programs::*;
use flo_stream::*;
use flo_binding::*;
use flo_canvas::scenery::*;
use flo_canvas_events::*;

use std::sync::*;

///
/// Creates a drawing window in a scene with the specified entity ID
///
/// The software renderer has no hardware layer, so it responds directly to `DrawingWindowRequest`
///
pub fn create_software_draw_window_program(scene: &Arc<Scene>, program_id: SubProgramId, initial_size: (u64, u64)) -> Result<(), ConnectionError> {
    // Create the window in context
    scene.add_subprogram(program_id, move |drawing_window_requests, context| {
        // Create the publisher to send the render actions to the stream
        let title               = bind("flo_draw".to_string());
        let fullscreen          = bind(false);
        let has_decorations     = bind(true);
        let mouse_pointer       = bind(MousePointer::SystemDefault);
        let size                = bind(initial_size);

        let window_properties   = WindowProperties { 
            title:              BindRef::from(title.clone()), 
            fullscreen:         BindRef::from(fullscreen.clone()), 
            has_decorations:    BindRef::from(has_decorations.clone()), 
            mouse_pointer:      BindRef::from(mouse_pointer.clone()), 
            size:               BindRef::from(size.clone()),
        };
        let mut event_publisher = Publisher::new(1000);

        // Create a stream for publishing render requests
        let (drawing_sender, drawing_receiver) = mpsc::channel(5);

        // Create a window that subscribes to the publisher (we do this outside of the main 'async' loop so this has happened on return)
        // If the window is not created immediately, there may be a race condition if `StopWhenAllWindowsClosed` is sent 
        let winit_thread    = winit_thread();
        winit_thread.send_event(WinitThreadEvent::CreateDrawingWindow(drawing_receiver.boxed(), event_publisher.republish(), window_properties.into()));

        async move {
            // Run the main event loop
            let mut drawing_window_requests = drawing_window_requests;
            let mut drawing_sender          = drawing_sender;

            while let Some(request) = drawing_window_requests.next().await {
                let request: DrawingWindowRequest = request;

                match request {
                    DrawingWindowRequest::Draw(DrawingRequest::Draw(drawing)) => {
                        if drawing_sender.send(drawing).await.is_err() {
                            // This entity is finished if the window finishes
                            break;
                        }
                    }

                    DrawingWindowRequest::Redraw => {
                        // Trigger a redraw by sending an empty request
                        drawing_sender.send(Arc::new(vec![])).await.ok();
                    }

                    DrawingWindowRequest::SendEvents(channel_target) => {
                        let mut subscriber = event_publisher.subscribe();

                        context.send_message(SceneControl::start_program(SubProgramId::new(), move |_: InputStream<()>, context| {
                            async move {
                                let events_target = context.send(channel_target).ok();

                                if let Some(mut events_target) = events_target {
                                    // Pass on events to everything that's listening, until the channel starts generating errors
                                    while let Some(event) = subscriber.next().await {
                                        let result = events_target.send(event).await;

                                        if result.is_err() {
                                            break;
                                        }
                                    }
                                }
                            }
                        }, 0)).await.ok();
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
                }
            }
        }
    }, 20);

    Ok(())
}
