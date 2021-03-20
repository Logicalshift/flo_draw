use super::events::*;
use super::glutin_thread::*;
use super::render_window::*;
use super::window_properties::*;
use super::glutin_thread_event::*;

use flo_canvas::*;
use flo_stream::*;
use flo_binding::*;
use flo_render::*;
use flo_render_canvas::*;

use ::desync::*;

use futures::prelude::*;
use futures::task::{Poll, Context};

use std::mem;
use std::pin::*;
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
    let (width, height)     = window_properties.size().get();

    // Create the canvas
    let canvas              = Canvas::new();
    canvas.draw(|gc| {
        // Default window layout is 1:1 for the requested window size
        gc.clear_canvas(Color::Rgba(1.0, 1.0, 1.0, 1.0));
        gc.canvas_height(height as _);
        gc.center_region(0.0, 0.0, width as _, height as _);
    });

    // Get the stream of drawing instructions (and gather them into batches)
    let canvas_stream       = canvas.stream();
    let canvas_stream       = drawing_with_laid_out_text(canvas_stream);
    let canvas_stream       = drawing_with_text_as_paths(canvas_stream);
    let canvas_stream       = BatchedStream { stream: Some(canvas_stream), frame_count: 0, waiting: vec![] };

    // Create the events stream
    let events              = create_canvas_window_from_stream(canvas_stream, window_properties);

    // Return the result
    (canvas, events)
}

///
/// Creates a canvas window that will render a stream of drawing instructions
///
pub fn create_canvas_window_from_stream<'a, DrawStream: 'static+Send+Unpin+Stream<Item=Vec<Draw>>, TProperties: 'a+FloWindowProperties>(canvas_stream: DrawStream, window_properties: TProperties) -> impl Clone+Send+Stream<Item=DrawEvent> {
    // Create a static copy of the window properties bindings
    let window_properties               = WindowProperties::from(&window_properties);

    // Create a render window
    let (render_actions, render_events) = create_render_window(window_properties);

    // Create a canvas renderer
    let renderer                        = CanvasRenderer::new();
    let renderer                        = RendererState { renderer: renderer, window_transform: None, scale: 1.0, width: 1.0, height: 1.0 };
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
        let render_events       = render_events.ready_chunks(1000);
        let mut canvas_updates  = CanvasUpdateStream {
            draw_stream:            Some(canvas_stream),
            event_stream:           render_events,
            waiting_frame_count:    0
        };

        // The window transform is used to track pointer events: it's invalidated when the size changes or the canvas is updated
        let mut window_transform            = None;
        let mut window_transform_invalid    = false;

        loop {
            // Retrieve the next canvas update
            match canvas_updates.next().await {
                Some(CanvasUpdate::DrawEvents(events)) => {
                    // Update the window transform if it is invalidated
                    if window_transform_invalid {
                        window_transform            = renderer.future_sync(|state| async move { state.window_transform }.boxed()).await.unwrap();
                        window_transform_invalid    = false;
                    }

                    // Process the events
                    for evt in events.iter() {
                        // Republish the event (adding the location on the canvas if necessary)
                        match evt {
                            DrawEvent::Pointer(action, pointer_id, pointer_state) => {
                                let mut pointer_state = pointer_state.clone();
                                
                                if let Some(window_transform) = &window_transform {
                                    let (x, y)                          = pointer_state.location_in_window;
                                    let (x, y)                          = (x as _, y as _);
                                    let (cx, cy)                        = window_transform.transform_point(x, y);
                                    pointer_state.location_in_canvas    = Some((cx as _, cy as _));
                                }

                                canvas_events.publish(DrawEvent::Pointer(*action, *pointer_id, pointer_state)).await;
                            }

                            _ => { canvas_events.publish(evt.clone()).await; }
                        }

                        // Closing the window immediately terminates the event loop, a new frame event reduces the waiting frame count
                        match evt {
                            DrawEvent::Closed       => { return; }
                            DrawEvent::NewFrame     => { if canvas_updates.waiting_frame_count > 0 { canvas_updates.waiting_frame_count -= 1; } },
                            DrawEvent::Redraw       => { window_transform_invalid = true; }
                            DrawEvent::Resize(_, _) => { window_transform_invalid = true; }
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
                        let window_transform    = state.update_window_transform();
                        canvas_events.publish(DrawEvent::CanvasTransform(window_transform)).await;

                        // Send the render actions to the window once they're ready
                        event_actions.publish(render_actions).await;
                    }.boxed());

                    window_transform_invalid = true;
                    
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
    window_events
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

///
/// Stream that takes a canvas stream and batches as many draw requests as possible
///
struct BatchedStream<TStream>
where TStream: Stream<Item=Draw> {
    /// Items that have been fetched and are waiting to be send
    waiting: Vec<TStream::Item>,

    /// The number of times StartFrame has been called
    frame_count: usize,

    // Stream of individual draw events
    stream: Option<TStream>
}

impl<TStream> Stream for BatchedStream<TStream>
where TStream: Unpin+Stream<Item=Draw> {
    type Item = Vec<TStream::Item>;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context) -> Poll<Option<Vec<TStream::Item>>> {
        let this        = self.get_mut();
        let this_stream = &mut this.stream;
        let waiting     = &mut this.waiting;
        let frame_count = &mut this.frame_count;

        match this_stream {
            None                =>  Poll::Ready(None), 
            Some(stream) => {
                // Poll the canvas stream until there are no more items to fetch
                let mut batch           = mem::take(waiting);
                let mut frame_offset    = 0;

                loop {
                    // Fill up the batch
                    match stream.poll_next_unpin(context) {
                        Poll::Ready(None)       => {
                            *this_stream = None;
                            break;
                        }

                        Poll::Ready(Some(Draw::StartFrame)) => {
                            *frame_count += 1;
                            batch.push(Draw::StartFrame);
                        }

                        Poll::Ready(Some(Draw::ShowFrame)) => {
                            if *frame_count > 0 {
                                *frame_count -= 1;
                            }

                            batch.push(Draw::ShowFrame);

                            if *frame_count == 0 {
                                frame_offset = batch.len();
                            }
                        }

                        Poll::Ready(Some(Draw::ClearCanvas(colour))) => {
                            *frame_count = 0;
                            batch.push(Draw::ClearCanvas(colour));
                        }

                        Poll::Ready(Some(draw)) => {
                            batch.push(draw)
                        }

                        Poll::Pending           => {
                            break;
                        }
                    }
                }

                if batch.len() == 0 && this_stream.is_none() {
                    // Stream finished with no more items
                    Poll::Ready(None)
                } else if batch.len() == 0 && this_stream.is_some() {
                    // No items were fetched for this batch
                    Poll::Pending
                } else {
                    // Batched up some drawing commands
                    if *frame_count == 0 {
                        // Not paused on a frame
                        Poll::Ready(Some(batch))
                    } else {
                        // Draw everything up until the most recent 'ShowFrame'
                        *waiting = batch.split_off(frame_offset);

                        if batch.len() == 0 {
                            Poll::Pending
                        } else {
                            Poll::Ready(Some(batch))
                        }
                    }
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
