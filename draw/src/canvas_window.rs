use super::draw_event::*;
use super::glutin_thread::*;
use super::render_window::*;
use super::glutin_thread_event::*;

use flo_canvas::*;
use flo_stream::*;
use flo_render::*;
use flo_render_canvas::*;

use ::desync::*;

use futures::future;
use futures::future::{Either};
use futures::prelude::*;
use futures::task::{Poll, Context};

use std::pin::*;
use std::sync::*;

///
/// Structure used to store the current state of the canvas renderer
///
struct RendererState {
    /// The renderer for the canvas
    renderer:       CanvasRenderer,

    /// The scale factor of the canvas
    scale:          f64,

    /// The width of the canvas
    width:          f64,

    /// The height of the canvas
    height:         f64,
}

///
/// Creates a canvas that will render to a window
///
pub fn create_canvas_window() -> Canvas {
    let (canvas, _events) = create_canvas_window_with_events();

    // Dropping the events will stop the window from blocking when they're not handled
    canvas
}

///
/// Creates a canvas that will render to a window, along with a stream of events from that window
///
pub fn create_canvas_window_with_events() -> (Canvas, impl Clone+Send+Stream<Item=DrawEvent>) {
    // Create the canvas
    let canvas                          = Canvas::new();

    // Create a render window
    let (render_actions, window_events) = create_render_window("flo_draw canvas");

    // Get the stream of drawing instructions (and gather them into batches)
    let canvas_stream                   = canvas.stream();
    let mut canvas_stream               = BatchedStream { stream: Some(canvas_stream) };

    // Create a canvas renderer
    let renderer                        = CanvasRenderer::new();
    let renderer                        = RendererState { renderer: renderer, scale: 1.0, width: 1.0, height: 1.0 };
    let renderer                        = Arc::new(Desync::new(renderer));
    let mut render_events               = window_events.clone();

    // Run the main canvas event loop as a process on the glutin thread
    glutin_thread().send_event(GlutinThreadEvent::RunProcess(Box::new(move || async move {
        // Handle events until the first 'redraw' event arrives (or stop if closed)
        loop {
            if let Some(event) = render_events.next().await {
                if let DrawEvent::Redraw = event {
                    // Begin the main event loop
                    // We've read nothing from the canvas yet so we can drop this event as the first canvas read will trigger a redraw anyway
                    break;
                }

                if let DrawEvent::Closed = event {
                    // Stop if the window is closed
                    return;
                }

                // Handle the next event (until the first 'redraw', we're receiving things like the window size in preparation for the next event)
                let mut event_actions = render_actions.republish();
                renderer.future_sync(move |state| async move { handle_window_event(state, event, &mut event_actions).await; }.boxed()).await.ok();
            } else {
                // Ran out of events
                return;
            }
        }

        // For the main event loop, we're always processing the window events, but alternate between reading from the canvas 
        // and waiting for the frame to render. We stop once there are no more events.
        let mut render_events   = BatchedStream { stream: Some(render_events) };
        let mut next_event      = render_events.next();
        let mut next_drawing    = canvas_stream.next();

        loop {
            // Waiting for a rendering task
            loop {
                // The next action is either an event from the window or a drawing request
                let next_action = future::select(next_event, next_drawing).await;

                match next_action {
                    Either::Left((None, _)) => {
                        // The main event stream has finished, so it's time to stop
                        return; 
                    }

                    Either::Right((None, waiting_for_events)) => {
                        // The canvas stream has finished: switch to just processing events until they run out
                        next_event = waiting_for_events;
                        loop {
                            if let Some(events) = next_event.await {
                                // Received an event
                                if events.iter().any(|evt| evt == &DrawEvent::Closed) {
                                    // The window is closed: stop processing events
                                    return;
                                }

                                // Process the event while we wait for the frame to render
                                let mut event_actions = render_actions.republish();
                                renderer.future_desync(move |state| async move { 
                                    for event in events.into_iter() {
                                        handle_window_event(state, event, &mut event_actions).await; 
                                    }
                                }.boxed());

                                // Fetch the next event
                                next_event = render_events.next();
                            } else {
                                // No more events: stop the renderer loop
                                return;
                            }
                        }
                    }

                    Either::Left((Some(events), waiting_for_drawing))  => {
                        // Received an event to process with the renderer
                        if events.iter().any(|evt| evt == &DrawEvent::Closed) {
                            // Stop if the window is closed
                            return;
                        }

                        let mut event_actions = render_actions.republish();
                        renderer.future_desync(move |state| async move { 
                            for event in events.into_iter() {
                                handle_window_event(state, event, &mut event_actions).await; 
                            }
                        }.boxed());

                        // Continue processing events
                        next_event      = render_events.next();
                        next_drawing    = waiting_for_drawing;
                    }

                    Either::Right((Some(drawing), waiting_for_events)) => {
                        // Received some drawing commands to forward to the canvas (which has rendered its previous frame)
                        let mut event_actions = render_actions.republish();
                        renderer.future_desync(move |state| async move {
                            // Wait for any pending render actions to clear the queue before trying to generate new ones
                            event_actions.when_empty().await;

                            // Ask the renderer to process the drawing instructions into render instructions
                            let render_actions = state.renderer.draw(drawing.into_iter()).collect::<Vec<_>>().await;

                            // Send the render actions to the window once they're ready
                            event_actions.publish(render_actions).await;
                        }.boxed());

                        // Continue processing events
                        next_event      = waiting_for_events;
                        next_drawing    = canvas_stream.next();

                        // Break out of the loop, to move to the 'wait for the next frame' state
                        break;
                    }
                }
            }

            // When we exit the above loop, we've dispatched a frame and want to monitor events until we get a 'newframe' event
            // This stops us from trying to queue more rendering instructions while the previous set are still being processed, so we don't start to get behind when the renderer lags behind the canvas
            loop {
                if let Some(events) = next_event.await {
                    // Received an event
                    let is_new_frame = events.iter().any(|evt| evt == &DrawEvent::NewFrame);

                    if events.iter().any(|evt| evt == &DrawEvent::Closed) {
                        // The window is closed: stop processing events
                        return;
                    }

                    // Process the event while we wait for the frame to render
                    let mut event_actions = render_actions.republish();
                    renderer.future_desync(move |state| async move { 
                        for event in events.into_iter() {
                            handle_window_event(state, event, &mut event_actions).await; 
                        }
                    }.boxed());

                    // Fetch the next event
                    next_event = render_events.next();

                    if is_new_frame {
                        // The frame we were waiting for has renderered: go back to processing canvas drawing instructions
                        break;
                    }
                } else {
                    // No more events: stop the renderer loop
                    return;
                }
            }
        }
    }.boxed_local())));

    // Return the events
    (canvas, window_events)
}

///
/// Handles an event from the window
///
fn handle_window_event<'a>(state: &'a mut RendererState, event: DrawEvent, render_actions: &'a mut Publisher<Vec<RenderAction>>) -> impl 'a+Send+Future<Output=()> {
    async move {
        match event {
            DrawEvent::Redraw                   => { 
                // Wait for any pending render actions to clear the queue before trying to generate new ones
                render_actions.when_empty().await;

                // Drawing nothing will regenerate the current contents of the renderer
                let redraw = state.renderer.draw(vec![].into_iter()).collect::<Vec<_>>().await;
                render_actions.publish(redraw).await;
            },

            DrawEvent::Scale(new_scale)         => {
                state.scale = new_scale;

                let width           = state.width as f32;
                let height          = state.height as f32;
                let scale           = state.scale as f32;

                state.renderer.set_viewport(0.0..width, 0.0..height, width, height, scale); 
            }

            DrawEvent::Resize(width, height)    => { 
                state.width         = width;
                state.height        = height;

                let width           = state.width as f32;
                let height          = state.height as f32;
                let scale           = state.scale as f32;

                state.renderer.set_viewport(0.0..width, 0.0..height, width, height, scale); 
            }

            DrawEvent::NewFrame                 => {

            }

            DrawEvent::Closed                   => {

            }
        }
    }
}

///
/// Stream that takes a canvas stream and batches as many draw requests as possible
///
struct BatchedStream<TStream>
where TStream: Stream {
    // Stream of individual draw events
    stream: Option<TStream>
}

impl<TStream> Stream for BatchedStream<TStream>
where TStream: Unpin+Stream {
    type Item = Vec<TStream::Item>;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context) -> Poll<Option<Vec<TStream::Item>>> {
        match &mut self.stream {
            None                =>  Poll::Ready(None), 
            Some(stream) => {
                // Poll the canvas stream until there are no more items to fetch
                let mut batch = vec![];

                loop {
                    // Fill up the batch
                    match stream.poll_next_unpin(context) {
                        Poll::Ready(None)       => {
                            self.stream = None;
                            break;
                        }

                        Poll::Ready(Some(draw)) => {
                            batch.push(draw)
                        }

                        Poll::Pending           => {
                            break;
                        }
                    }
                }

                if batch.len() == 0 && self.stream.is_none() {
                    // Stream finished with no more items
                    Poll::Ready(None)
                } else if batch.len() == 0 && self.stream.is_some() {
                    // No items were fetched for this batch
                    Poll::Pending
                } else {
                    // Batched up some drawing commands
                    Poll::Ready(Some(batch))
                }
            }
        }
    }
}
