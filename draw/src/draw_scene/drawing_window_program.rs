use futures::prelude::*;
use futures::{pin_mut};
use futures::task::{Poll, Context};

use flo_scene::*;
use flo_scene::programs::*;
use flo_stream::*;
use flo_canvas::*;
use flo_canvas::scenery::*;
use flo_canvas_events::*;
use flo_render_canvas::*;

use once_cell::sync::{Lazy};

use std::pin::*;
use std::sync::*;

///
/// Combines rendering and event messages into one enum
///
#[derive(Debug)]
enum DrawingOrEvent {
    Drawing(Vec<DrawingWindowRequest>),
    Event(Vec<DrawEventRequest>),
}

impl SceneMessage for DrawingOrEvent { }

static FILTER_DRAWING_WINDOW_REQUEST: Lazy<FilterHandle> = Lazy::new(|| FilterHandle::for_filter(|drawing_window_requests| {
    drawing_window_requests.ready_chunks(100)
        .map(|requests| DrawingOrEvent::Drawing(requests))
}));
static FILTER_DRAWING_EVENT_REQUEST: Lazy<FilterHandle> = Lazy::new(|| FilterHandle::for_filter(|drawing_event_requests| {
    drawing_event_requests.ready_chunks(100)
        .map(|requests| DrawingOrEvent::Event(requests))
}));

///
/// Stream that reads instructions from the drawing or event stream
///
/// Drawing stream may be suspended while we wait for new frames, and the event stream has priority at all other times
///
struct DrawingEventStream<TDrawStream, TEventStream>
where
    TDrawStream:    Unpin + Stream<Item=DrawingOrEvent>,
    TEventStream:   Unpin + Stream<Item=DrawingOrEvent>,
{
    // If set to true, the stream will not attempt to poll the drawing stream
    waiting_for_new_frame: bool,

    /// The drawing stream, or None if it has been closed
    draw_stream: Option<TDrawStream>,

    /// The event stream, or None if it has been closed
    event_stream: Option<TEventStream>,
}

///
/// Structure used to store the current state of the canvas renderer
///
struct RendererState {
    /// The renderer for the canvas
    renderer:       CanvasRenderer,

    /// The transformation from window coordinates to canvas coordinates
    window_transform: Option<Transform2D>,

    /// The scale factor of the canvas
    scale:          f64,

    /// The width of the canvas
    width:          f64,

    /// The height of the canvas
    height:         f64,
}

impl<TDrawStream, TEventStream> Stream for DrawingEventStream<TDrawStream, TEventStream>
where
    TDrawStream:    Unpin + Stream<Item=DrawingOrEvent>,
    TEventStream:   Unpin + Stream<Item=DrawingOrEvent>,
{
    type Item = DrawingOrEvent;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // First, see if the event stream has anything for us, and return the event from there if it exists
        if let Some(event_stream) = &mut self.event_stream {
            let event_poll_result = event_stream.poll_next_unpin(context);

            match event_poll_result {
                Poll::Ready(Some(event))    => { return Poll::Ready(Some(event)); }
                Poll::Ready(None)           => { self.event_stream = None; }
                Poll::Pending               => { }
            }
        }

        // Check the draw stream if we're not waiting for a frame
        if !self.waiting_for_new_frame {
            if let Some(draw_stream) = &mut self.draw_stream {
                let draw_poll_result = draw_stream.poll_next_unpin(context);

                match draw_poll_result {
                    Poll::Ready(Some(event))    => { return Poll::Ready(Some(event)); }
                    Poll::Ready(None)           => { self.draw_stream = None; }
                    Poll::Pending               => { }
                }
            }
        }

        // If both streams are done, indicate that we're finished
        if self.draw_stream.is_none() && self.event_stream.is_none() {
            return Poll::Ready(None);
        }

        // Waiting on one or both of the streams
        Poll::Pending
    }
}

///
/// Handles an event from the window
///
/// The return value is any extra events to synthesize as a result of the initial event
///
fn handle_window_event<'a, SendFuture, SendRenderActionsFn>(state: &'a mut RendererState, event: DrawEvent, send_render_actions: &'a mut SendRenderActionsFn) -> impl 'a + Send + Future<Output=Vec<DrawEvent>> 
where 
    SendRenderActionsFn:    Send + Fn(Vec<RenderAction>) -> SendFuture,
    SendFuture:             Send + Future<Output=()> 
{
    async move {
        match event {
            DrawEvent::Redraw                   => { 
                // Drawing nothing will regenerate the current contents of the renderer
                let redraw = state.renderer.draw(vec![].into_iter()).collect::<Vec<_>>().await;
                send_render_actions(redraw).await;

                let window_transform    = state.update_window_transform();
                vec![DrawEvent::CanvasTransform(window_transform)]
            },

            DrawEvent::Scale(new_scale)         => {
                state.scale = new_scale;

                let width           = state.width as f32;
                let height          = state.height as f32;
                let scale           = state.scale as f32;

                state.renderer.set_viewport(0.0..width, 0.0..height, width, height, scale);

                vec![]
            }

            DrawEvent::Resize(width, height)    => { 
                state.width         = width;
                state.height        = height;

                let width           = state.width as f32;
                let height          = state.height as f32;
                let scale           = state.scale as f32;

                state.renderer.set_viewport(0.0..width, 0.0..height, width, height, scale); 

                vec![]
            }

            DrawEvent::NewFrame                 => { vec![] }
            DrawEvent::Closed                   => { vec![] }
            DrawEvent::CanvasTransform(_)       => { vec![] }
            DrawEvent::Pointer(_, _, _)         => { vec![] }
            DrawEvent::KeyDown(_, _)            => { vec![] }
            DrawEvent::KeyUp(_, _)              => { vec![] }
        }
    }
}

impl RendererState {
    ///
    /// Updates the window transform for this state
    ///
    fn update_window_transform(&mut self) -> Transform2D {
        // Fetch the window tranform from the canvas, and invert it to get the transform from window coordinates to canvas coordinates
        let window_transform    = self.renderer.get_window_transform().invert().unwrap();

        // Window coordinates are inverted compared to canvas coordinates
        let window_transform    = Transform2D::scale(1.0, -1.0) * window_transform;
        let window_transform    = window_transform * Transform2D::translate(0.0, -self.height as _);

        // Update the value of the transform in the state
        self.window_transform   = Some(window_transform);
        window_transform
    }

    ///
    /// Performs a drawing action and passes it on to the render target
    ///
    async fn draw(&mut self, draw_actions: impl Send + Iterator<Item=&Draw>, render_target: &mut Pin<&mut (impl 'static + Sink<RenderWindowRequest>)>) {
        let render_actions = self.renderer.draw(draw_actions.cloned()).collect::<Vec<_>>().await;
        render_target.send(RenderWindowRequest::Render(RenderRequest::Render(render_actions))).await.ok();
    }
}

///
/// Creates a drawing window that sends render requests to the specified target
///
pub fn create_drawing_window_program(scene: &Arc<Scene>, program_id: SubProgramId, render_target_program: SubProgramId) -> Result<(), ConnectionError> {
    // Create the window in the scene
    scene.add_subprogram(
        program_id, 
        move |drawing_window_requests, context| async move {
            // We relay events via our own event publisher
            let mut event_publisher = Publisher::new(1000);

            // Set up the renderer and window state
            let mut render_state = RendererState {
                renderer:           CanvasRenderer::new(),
                window_transform:   None,
                scale:              1.0,
                width:              1.0,
                height:             1.0,
            };

            // Request the events from the render target
            let render_target   = context.send::<RenderWindowRequest>(render_target_program);
            let render_target   = if let Ok(render_target) = render_target { render_target } else { return; };
            pin_mut!(render_target);
            render_target.send(RenderWindowRequest::SendEvents(program_id)).await.ok();

            // Initially the window is not ready to render (we need to wait for the first 'redraw' event)
            let mut ready_to_render             = false;
            let mut waiting_for_new_frame       = false;
            let mut drawing_since_last_frame    = false;
            let mut closed                      = false;

            // Pause the drawing using a start frame event
            render_state.draw(vec![Draw::StartFrame].iter(), &mut render_target).await;

            // Run the main event loop
            let mut messages = drawing_window_requests;
            while let Some(message) = messages.next().await {
                match message {
                    DrawingOrEvent::Drawing(drawing_list) => {
                        // Perform all the actions in a single frame
                        let mut combined_list   = vec![Arc::new(vec![Draw::StartFrame])];

                        // If we've rendered something and 'NewFrame' hasn't yet been generated, add an extra 'StartFrame' to suspend rendering until the last frame is finished
                        if waiting_for_new_frame && !drawing_since_last_frame {
                            drawing_since_last_frame = true;
                            combined_list.push(Arc::new(vec![Draw::StartFrame]));
                        }

                        for draw_msg in drawing_list {
                            match draw_msg {
                                DrawingWindowRequest::Draw(DrawingRequest::Draw(drawing)) => {
                                    // Send the drawing to the renderer
                                    combined_list.push(drawing);
                                }

                                DrawingWindowRequest::CloseWindow => {
                                    // Just stop running when there's a 'close' request
                                    closed = true;
                                }

                                DrawingWindowRequest::SendEvents(target_program) => {
                                    // Output to the target program using another program
                                    let mut subscriber  = event_publisher.subscribe();
                                    let channel_target  = context.send::<DrawEvent>(target_program);

                                    if let Ok(channel_target) = channel_target {
                                        context.send_message(SceneControl::start_program(SubProgramId::new(), move |_: InputStream<()>, _| async move {
                                            // Pass on events to everything that's listening, until the channel starts generating errors
                                            let mut channel_target = channel_target;
                                            while let Some(event) = subscriber.next().await {
                                                let result = channel_target.send(event).await;

                                                if result.is_err() {
                                                    break;
                                                }
                                            }
                                        }, 0)).await.ok();
                                    }
                                }

                                DrawingWindowRequest::SetTitle(title)                   => { render_target.send(RenderWindowRequest::SetTitle(title)).await.ok(); },
                                DrawingWindowRequest::SetFullScreen(fullscreen)         => { render_target.send(RenderWindowRequest::SetFullScreen(fullscreen)).await.ok(); },
                                DrawingWindowRequest::SetHasDecorations(decorations)    => { render_target.send(RenderWindowRequest::SetHasDecorations(decorations)).await.ok(); },
                                DrawingWindowRequest::SetMousePointer(mouse_pointer)    => { render_target.send(RenderWindowRequest::SetMousePointer(mouse_pointer)).await.ok(); },
                            }
                        }

                        // Commit the frame. We'll add backpressure to new drawing events by not accepting them.
                        waiting_for_new_frame           = true;
                        // Can reimplement the TODO here by having another program that accepts the drawing events and does the conversion
                        // messages.waiting_for_new_frame  = true;         // TODO: we currently can't stop polling for drawing events here, so we need a way to reimplement this

                        combined_list.push(Arc::new(vec![Draw::ShowFrame]));
                        render_state.draw(combined_list.iter()
                            .flat_map(|item| item.iter()), &mut render_target).await;

                        // Update the window transform according to the drawing actions we processed
                        render_state.update_window_transform();
                    }

                    DrawingOrEvent::Event(event_list) => {
                        for evt_message in event_list.into_iter() {
                            let mut evt_message = evt_message;

                            match &evt_message {
                                // TODO: StartFrame/ShowFrame based on the 'NewFrame' event
                                
                                DrawEvent::Pointer(action, pointer_id, pointer_state) => {
                                    // Rewrite pointer events before republishing them
                                    let mut pointer_state = pointer_state.clone();
                                    
                                    if let Some(window_transform) = &render_state.window_transform {
                                        let (x, y)                          = pointer_state.location_in_window;
                                        let (x, y)                          = (x as _, y as _);
                                        let (cx, cy)                        = window_transform.transform_point(x, y);
                                        pointer_state.location_in_canvas    = Some((cx as _, cy as _));
                                    }

                                    evt_message = DrawEvent::Pointer(*action, *pointer_id, pointer_state);
                                }

                                DrawEvent::Redraw => {
                                    // When a redraw event arrives, we're ready to render from the renderer to the window
                                    if !ready_to_render {
                                        // Move to the 'ready to render' state
                                        ready_to_render = true;

                                        // Show the frame from the initial 'StartFrame' request
                                        render_state.draw(vec![Draw::ShowFrame].iter(), &mut render_target).await;
                                    }
                                },

                                DrawEvent::Closed => {
                                    // Close events terminate the loop (after we've finshed processing the events)
                                    closed = true;
                                }

                                DrawEvent::NewFrame => {
                                    // A new frame was displayed
                                    waiting_for_new_frame = false;

                                    if drawing_since_last_frame {
                                        // Finalize any drawing that occurred while we were waiting for the new frame to display
                                        waiting_for_new_frame = true;
                                        render_state.draw(vec![Draw::ShowFrame].iter(), &mut render_target).await;
                                        drawing_since_last_frame = false;
                                    }

                                    // TODO: add a way to add backpressure if we're waiting for a new frame
                                    //messages.waiting_for_new_frame = waiting_for_new_frame;
                                }

                                _ => { }
                            }

                            // Publish the event to any subscribers
                            event_publisher.publish(evt_message.clone()).await;

                            // Handle the next message
                            let context = &context;
                            handle_window_event(&mut render_state, evt_message, &mut move |render_actions| {
                                let render_target = context.send::<RenderWindowRequest>(render_target_program);

                                async move {
                                    if let Ok(mut render_target) = render_target {
                                        render_target.send(RenderWindowRequest::Render(RenderRequest::Render(render_actions))).await.ok();
                                    }
                                }
                            }).await;
                        }

                        // The entity stops when the window is closed
                        if closed {
                            break;
                        }
                    }
                }
            }

            // Shut down
            render_target.send(RenderWindowRequest::CloseWindow).await.ok();

            use std::mem;

            let when_closed = event_publisher.when_closed();

            // Drop the receivers
            mem::drop(messages);
            mem::drop(event_publisher);

            // Wait for the publisher to finish up
            when_closed.await;
        },
        100);

    // This window can accept a couple of converted messages
    scene.connect_programs((), StreamTarget::Filtered(*FILTER_DRAWING_WINDOW_REQUEST, program_id), StreamId::for_target::<DrawingWindowRequest>(program_id)).unwrap();
    scene.connect_programs((), StreamTarget::Filtered(*FILTER_DRAWING_EVENT_REQUEST, program_id), StreamId::for_target::<DrawEventRequest>(program_id)).unwrap();

    Ok(())
}
