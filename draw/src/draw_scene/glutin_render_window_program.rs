use crate::glutin::*;
use crate::window_properties::*;

use futures::prelude::*;
use futures::channel::mpsc;

use flo_scene::*;
use flo_scene::programs::*;
use flo_stream::*;
use flo_binding::*;
use flo_canvas_events::*;

use std::sync::*;

///
/// Creates a render window in a scene with the specified entity ID
///
pub fn create_glutin_render_window_program(scene: &Arc<Scene>, program_id: SubProgramId, initial_size: (u64, u64)) -> Result<impl Sink<RenderWindowRequest, Error=SceneSendError>, ConnectionError> {
    // This window can accept a couple of converted messages
    //context.convert_message::<RenderRequest, RenderWindowRequest>()?;
    //context.convert_message::<EventWindowRequest, RenderWindowRequest>()?;

    // Create the window in the scene
    scene.add_subprogram(
        program_id,
        move |render_window_requests, context| async move {
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
            let (render_sender, render_receiver) = mpsc::channel(5);

            // Create a window that subscribes to the publisher
            let glutin_thread   = glutin_thread();
            glutin_thread.send_event(GlutinThreadEvent::CreateRenderWindow(render_receiver.boxed(), event_publisher.republish(), window_properties.into()));

            // Run the main event loop
            let mut render_window_requests  = render_window_requests;
            let mut render_sender           = render_sender;

            while let Some(request) = render_window_requests.next().await {
                let request: RenderWindowRequest = request;

                match request {
                    RenderWindowRequest::Render(RenderRequest::Render(render)) => {
                        // Just pass render requests on to the render window
                        if render_sender.send(render).await.is_err() {
                            // This entity is finished if the window finishes
                            break;
                        }
                    }

                    RenderWindowRequest::SendEvents(channel_target) => {
                        // Run a subprogram to send the events to the target program
                        let subscriber = Mutex::new(Some(event_publisher.subscribe()));

                        context.send_message(SceneControl::start_program(SubProgramId::new(), move |_: InputStream<()>, context| {
                            let subscriber = subscriber.lock().unwrap().take();

                            async move {
                                if let Some(mut subscriber) = subscriber {
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
                            }
                        }, 20)).await.ok();
                    }

                    RenderWindowRequest::CloseWindow => {
                        // The window will close its publisher in response to the events stream being closed
                        render_sender.close().await.ok();

                        // Shut down the event publisher
                        use std::mem;
                        let when_closed = event_publisher.when_closed();
                        mem::drop(event_publisher);

                        // Finally, wait for the publisher to finish up
                        when_closed.await;
                        return;
                    }

                    RenderWindowRequest::SetTitle(new_title)                => { title.set(new_title); },
                    RenderWindowRequest::SetFullScreen(new_fullscreen)      => { fullscreen.set(new_fullscreen); },
                    RenderWindowRequest::SetHasDecorations(new_decorations) => { has_decorations.set(new_decorations); },
                    RenderWindowRequest::SetMousePointer(new_mouse_pointer) => { mouse_pointer.set(new_mouse_pointer); },
                }
            }
        },
        20);

    // Result is a stream to this program
    scene.send_to_scene::<RenderWindowRequest>(program_id)
}
