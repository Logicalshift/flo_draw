use futures::prelude::*;
use futures::stream;

use flo_scene::*;
use flo_stream::*;
use flo_canvas::*;
use flo_canvas_events::*;
use flo_render_canvas::*;

use std::sync::*;

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

///
/// Handles an event from the window
///
/// The return value is any extra events to synthesize as a result of the initial event
///
fn handle_window_event<'a, SendFuture, SendRenderActionsFn>(state: &'a mut RendererState, event: DrawEvent, send_render_actions: &'a mut SendRenderActionsFn) -> impl 'a + Send + Future<Output=Vec<DrawEvent>> 
where 
    SendRenderActionsFn:    Send + FnMut(Vec<RenderAction>) -> SendFuture,
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
    async fn draw(&mut self, draw_actions: impl Send + Iterator<Item=&Draw>, render_target: &mut (impl 'static + EntityChannel<Message=RenderWindowRequest, Response=()>)) {
        let render_actions = self.renderer.draw(draw_actions.cloned()).collect::<Vec<_>>().await;
        render_target.send_without_waiting(RenderWindowRequest::Render(RenderRequest::Render(render_actions))).await.ok();
    }
}

///
/// Creates a drawing window that sends render requests to the specified target
///
pub fn create_drawing_window_entity(context: &Arc<SceneContext>, entity_id: EntityId, render_target: impl 'static + EntityChannel<Message=RenderWindowRequest, Response=()>) -> Result<SimpleEntityChannel<DrawingWindowRequest, ()>, CreateEntityError> {
    // This window can accept a couple of converted messages
    context.convert_message::<DrawingRequest, DrawingWindowRequest>()?;
    context.convert_message::<EventWindowRequest, DrawingWindowRequest>()?;

    // Create the window in context
    context.create_entity(entity_id, move |context, drawing_window_requests| async move {
        let mut render_target       = render_target;

        // We relay events via our own event publisher
        let mut event_publisher = Publisher::new(1000);

        // Set up the renderer and window state
        let mut render_state        = RendererState {
            renderer:           CanvasRenderer::new(),
            window_transform:   None,
            scale:              1.0,
            width:              1.0,
            height:             1.0,
        };

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

        // Initially the window is not ready to render (we need to wait for the first 'redraw' event)
        let mut ready_to_render         = false;
        let mut closed                  = false;

        // Pause the drawing using a start frame event
        render_state.draw(vec![Draw::StartFrame].iter(), &mut render_target).await;

        // Run the main event loop
        let mut messages = messages;
        while let Some(message) = messages.next().await {
            match message {
                DrawingOrEvent::Drawing(drawing_list) => {
                    // Perform all the actions in a single frame
                    render_state.draw(vec![Draw::StartFrame].iter(), &mut render_target).await;

                    for draw_msg in drawing_list {
                        // Take the message
                        let (draw_msg, responder) = draw_msg.take();

                        match draw_msg {
                            DrawingWindowRequest::Draw(DrawingRequest::Draw(drawing)) => {
                                // Send the drawing to the renderer
                                render_state.draw(drawing.iter(), &mut render_target).await;

                                // Send the response once the drawing action has completed
                                responder.send(()).ok();
                            }

                            DrawingWindowRequest::SendEvents(event_channel) => {
                                let mut subscriber = event_publisher.subscribe();

                                context.run_in_background(async move {
                                    let mut channel_target = event_channel;

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
                    }

                    // Commit the frame
                    render_state.draw(vec![Draw::ShowFrame].iter(), &mut render_target).await;

                    // Update the window transform according to the drawing actions we processed
                    render_state.update_window_transform();
                }

                DrawingOrEvent::Event(event_list) => {
                    for evt_message in event_list.into_iter() {
                        // Take the message apart
                        let (mut evt_message, responder) = evt_message.take();

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

                            _ => { }
                        }

                        // Publish the event to any subscribers
                        event_publisher.publish(evt_message.clone()).await;

                        // Handle the next message
                        handle_window_event(&mut render_state, evt_message, &mut |render_actions| {
                            let send_rendering = render_target.send_without_waiting(RenderWindowRequest::Render(RenderRequest::Render(render_actions)));
                            async move {
                                send_rendering.await.ok();
                            }
                        }).await;

                        // Indicate that the event has been handled
                        responder.send(()).ok();
                    }

                    // The entity stops when the window is closed
                    if closed {
                        break;
                    }
                }
            }
        }
    })
}
