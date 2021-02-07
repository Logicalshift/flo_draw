use super::draw_event::*;
use super::glutin_thread::*;
use super::render_window::*;
use super::window_properties::*;
use super::glutin_thread_event::*;

use flo_canvas::*;
use flo_stream::*;
use flo_render::*;
use flo_render_canvas::*;

use ::desync::*;

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
pub fn create_canvas_window<'a, TProperties: 'a+FloWindowProperties>(window_properties: TProperties) -> Canvas {
    let (canvas, _events) = create_canvas_window_with_events(window_properties);

    // Dropping the events will stop the window from blocking when they're not handled
    canvas
}

///
/// Creates a canvas that will render to a window, along with a stream of events from that window
///
pub fn create_canvas_window_with_events<'a, TProperties: 'a+FloWindowProperties>(window_properties: TProperties) -> (Canvas, impl Clone+Send+Stream<Item=DrawEvent>) {
    // Create a static copy of the window properties bindings
    let window_properties               = WindowProperties::from(&window_properties);

    // Create the canvas
    let canvas                          = Canvas::new();

    // Create a render window
    let (render_actions, render_events) = create_render_window(window_properties);

    // Get the stream of drawing instructions (and gather them into batches)
    let canvas_stream                   = canvas.stream();
    let canvas_stream                   = BatchedStream { stream: Some(canvas_stream) };

    // Create a canvas renderer
    let renderer                        = CanvasRenderer::new();
    let renderer                        = RendererState { renderer: renderer, scale: 1.0, width: 1.0, height: 1.0 };
    let renderer                        = Arc::new(Desync::new(renderer));
    let mut render_events               = render_events;

    // We republish the events, so we can add our own canvas events
    let mut canvas_events               = Publisher::new(1000);
    let window_events                   = canvas_events.subscribe();

    // Run the main canvas event loop as a process on the glutin thread
    glutin_thread().send_event(GlutinThreadEvent::RunProcess(Box::new(move || async move {
        // Handle events until the first 'redraw' event arrives (or stop if closed)
        loop {
            if let Some(event) = render_events.next().await {
                canvas_events.publish(event.clone()).await;

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
                renderer.future_sync(move |state| async move { 
                    handle_window_event(state, event, &mut |actions| event_actions.publish(actions)).await; 
                }.boxed()).await.ok();
            } else {
                // Ran out of events
                return;
            }
        }

        // For the main event loop, we're always processing the window events, but alternate between reading from the canvas 
        // and waiting for the frame to render. We stop once there are no more events.
        let render_events       = BatchedStream { stream: Some(render_events) };
        let mut canvas_updates  = CanvasUpdateStream {
            draw_stream:            Some(canvas_stream),
            event_stream:           render_events,
            waiting_frame_count:    0
        };

        loop {
            // Retrieve the next canvas update
            match canvas_updates.next().await {
                Some(CanvasUpdate::DrawEvents(events)) => {
                    // Process the events
                    for evt in events.iter() {
                        canvas_events.publish(evt.clone()).await;

                        // Closing the window immediately terminates the event loop, a new frame event reduces the waiting frame count
                        match evt {
                            DrawEvent::Closed       => { return; }
                            DrawEvent::NewFrame     => { if canvas_updates.waiting_frame_count > 0 { canvas_updates.waiting_frame_count -= 1; } },
                            _                       => { }
                        }
                    }

                    // Handle the events on the renderer thread
                    let mut event_actions   = render_actions.republish();
                    let new_events          = renderer.future_sync(move |state| async move { 
                        let mut new_events = vec![];

                        for event in events.into_iter() {
                            // Handle the event
                            new_events.extend(handle_window_event(state, event, &mut |actions| event_actions.publish(actions)).await);
                        }

                        new_events
                    }.boxed()).await.unwrap_or_else(|_| vec![]);

                    // Send any new events to the canvas events publisher
                    for new_event in new_events.into_iter() {
                        canvas_events.publish(new_event).await;
                    }
                }

                Some(CanvasUpdate::Drawing(drawing)) => {
                     // Received some drawing commands to forward to the canvas (which has rendered its previous frame)
                    let mut event_actions   = render_actions.republish();
                    let mut canvas_events   = canvas_events.republish();

                    renderer.future_desync(move |state| async move {
                        // Wait for any pending render actions to clear the queue before trying to generate new ones
                        event_actions.when_empty().await;

                        // Ask the renderer to process the drawing instructions into render instructions
                        let render_actions = state.renderer.draw(drawing.into_iter()).collect::<Vec<_>>().await;

                        // Send an update that the canvas transform has changed
                        canvas_events.publish(DrawEvent::CanvasTransform(state.renderer.get_window_transform())).await;

                        // Send the render actions to the window once they're ready
                        event_actions.publish(render_actions).await;
                    }.boxed());
                    
                    // Don't read any more from the canvas until the frame has finished rendering
                    canvas_updates.waiting_frame_count += 1;
                }

                None => {
                    // The main event loop has finished: stop processing events
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
/// The return value is any extra events to synthesize as a result of the initial event
///
fn handle_window_event<'a, SendFuture, SendRenderActionsFn>(state: &'a mut RendererState, event: DrawEvent, send_render_actions: &'a mut SendRenderActionsFn) -> impl 'a+Send+Future<Output=Vec<DrawEvent>> 
where 
SendRenderActionsFn:    Send+FnMut(Vec<RenderAction>) -> SendFuture,
SendFuture:             Send+Future<Output=()> {
    async move {
        match event {
            DrawEvent::Redraw                   => { 
                // Drawing nothing will regenerate the current contents of the renderer
                let redraw = state.renderer.draw(vec![].into_iter()).collect::<Vec<_>>().await;
                send_render_actions(redraw).await;

                vec![DrawEvent::CanvasTransform(state.renderer.get_window_transform())]
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

///
/// Update events that can be passed to the canvas
///
enum CanvasUpdate {
    /// New drawing actions
    Drawing(Vec<Draw>),

    /// Events from the window
    DrawEvents(Vec<DrawEvent>)
}

///
/// Stream that generates canvas update events
///
/// We avoid reading drawing events if we're waiting for a frame to render (this means that if the canvas
/// turns out to be expensive to render, we won't waste time tessellating frames that will never actually
/// show up)
///
struct CanvasUpdateStream<TDrawStream, TEventStream> {
    draw_stream:            Option<TDrawStream>,
    event_stream:           TEventStream,

    waiting_frame_count:    usize
}

impl<TDrawStream, TEventStream> Stream for CanvasUpdateStream<TDrawStream, TEventStream> 
where 
TDrawStream:    Unpin+Stream<Item=Vec<Draw>>,
TEventStream:   Unpin+Stream<Item=Vec<DrawEvent>> {
    type Item = CanvasUpdate;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Events get priority
        match self.event_stream.poll_next_unpin(context) {
            Poll::Ready(Some(events))   => { return Poll::Ready(Some(CanvasUpdate::DrawEvents(events))); }
            Poll::Ready(None)           => { return Poll::Ready(None); }
            Poll::Pending               => { }
        }

        // We only poll the canvas stream if we're not waiting for frame events
        if self.waiting_frame_count == 0 {
            // The canvas stream can get closed, in which case it will be set to 'None'
            if let Some(draw_stream) = self.draw_stream.as_mut() {
                match draw_stream.poll_next_unpin(context) {
                    Poll::Ready(Some(drawing))  => { return Poll::Ready(Some(CanvasUpdate::Drawing(drawing))); }
                    Poll::Ready(None)           => { self.draw_stream = None; }
                    Poll::Pending               => { }
                }
            }
        }

        // No events are ready yet
        Poll::Pending
    }
}
