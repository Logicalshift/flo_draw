use crate::events::*;
use crate::render_window::*;
use crate::window_properties::*;
use crate::draw_scene::*;

use flo_canvas::*;
use flo_canvas::scenery::*;
use flo_binding::*;
use flo_scene::*;

use futures::prelude::*;
use futures::channel::mpsc;
use futures::task::{Poll, Context};

use std::mem;
use std::pin::*;
use std::sync::*;
use std::time::{Duration, Instant};

const MAX_BATCH_TIME: Duration = Duration::from_nanos(1_000_000_000 / 60);

///
/// Creates a drawing target that will render to a window
///
pub fn create_drawing_window<'a, TProperties>(window_properties: TProperties) -> DrawingTarget 
where
    TProperties: 'a + FloWindowProperties,
{
    let (target, _events) = create_drawing_window_with_events(window_properties);

    // Dropping the events will stop the window from blocking when they're not handled
    target
}

///
/// Creates a drawing target that will render to a window, along with a stream of events from that window
///
pub fn create_drawing_window_with_events<'a, TProperties>(window_properties: TProperties) -> (DrawingTarget, impl Send + Stream<Item=DrawEvent>) 
where
    TProperties: 'a + FloWindowProperties,
{
    let (width, height)     = window_properties.size().get();

    // Create the canvas
    let (target, stream)    = DrawingTarget::new();
    target.draw(|gc| {
        // Default window layout is 1:1 for the requested window size
        gc.clear_canvas(Color::Rgba(1.0, 1.0, 1.0, 1.0));
        gc.canvas_height(height as _);
        gc.center_region(0.0, 0.0, width as _, height as _);
    });

    // Get the stream of drawing instructions (and gather them into batches)
    let target_stream       = stream;
    let target_stream       = drawing_without_dashed_lines(target_stream);
    let target_stream       = drawing_with_laid_out_text(target_stream);
    let target_stream       = drawing_with_text_as_paths(target_stream);
    let target_stream       = BatchedStream { stream: Some(target_stream), frame_count: 0, waiting: vec![] };

    // Create the events stream
    let events              = create_drawing_window_from_stream(target_stream, window_properties);

    // Return the result
    (target, events)
}

///
/// Creates a canvas that will render to a window
///
/// Canvases differ from drawing targets in that they store the vector representation of what they're drawing, and
/// can send their rendering to multiple targets if necessary
///
pub fn create_canvas_window<'a, TProperties: 'a+FloWindowProperties>(window_properties: TProperties) -> Canvas {
    let (canvas, _events) = create_canvas_window_with_events(window_properties);

    // Dropping the events will stop the window from blocking when they're not handled
    canvas
}

///
/// Creates a drawing target that will render to a window, along with a stream of events from that window
///
pub fn create_canvas_window_with_events<'a, TProperties>(window_properties: TProperties) -> (Canvas, impl Send + Sync + Stream<Item=DrawEvent>) 
where
    TProperties: 'a + FloWindowProperties,
{
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
    let canvas_stream       = drawing_without_dashed_lines(canvas_stream);
    let canvas_stream       = drawing_with_laid_out_text(canvas_stream);
    let canvas_stream       = drawing_with_text_as_paths(canvas_stream);
    let canvas_stream       = BatchedStream { stream: Some(canvas_stream), frame_count: 0, waiting: vec![] };

    // Create the events stream
    let events              = create_drawing_window_from_stream(canvas_stream, window_properties);

    // Return the result
    (canvas, events)
}

///
/// Creates a drawing window that will render a stream of drawing instructions
///
pub fn create_drawing_window_from_stream<'a, DrawStream, TProperties>(canvas_stream: DrawStream, window_properties: TProperties) -> impl Send + Stream<Item=DrawEvent>
where
    DrawStream:  'static + Send + Unpin + Stream<Item=Vec<Draw>>,
    TProperties: 'a + FloWindowProperties,
{
    let properties              = WindowProperties::from(&window_properties);

    // Create a new render window entity
    let render_window_program   = SubProgramId::new();
    let drawing_window_program  = SubProgramId::new();
    let scene_context           = flo_draw_scene_context();

    create_render_window_sub_program(&scene_context, render_window_program, window_properties.size().get()).unwrap();
    create_drawing_window_program(&scene_context, drawing_window_program, render_window_program).unwrap();

    // Use a channel to get the events out of the program
    let (send_events, recv_events)  = mpsc::channel(20);
    let event_relay_program         = SubProgramId::new();
    scene_context.add_subprogram(event_relay_program,
        move |mut draw_events: InputStream<DrawEvent>, _| async move {
            let mut send_events = send_events;
            draw_events.allow_thread_stealing(true);

            while let Some(event) = draw_events.next().await {
                let is_closed = event == DrawEvent::Closed;

                match send_events.send(event).await {
                    Ok(())  => { },
                    Err(_)  => { break; }
                };

                if is_closed {
                    break;
                }
            }
        },
        0);

    // Pass events from the render stream onto the window using another entity (potentially this could be a background task for the render window entity?)
    let processing_subprogram = SubProgramId::new();
    scene_context.add_subprogram(processing_subprogram, move |_: InputStream<()>, context| {
        async move {
            let mut canvas_stream   = canvas_stream;
            let mut drawing_channel = context.send::<DrawingWindowRequest>(drawing_window_program).unwrap();

            // Send the window properties to the window
            send_window_properties::<DrawingWindowRequest>(&context, properties, drawing_window_program).await.ok();

            // Request event actions from the renderer to the relay program (which sends them on to the stream returned from this function)
            drawing_channel.send(DrawingWindowRequest::SendEvents(event_relay_program)).await.ok();

            // Main loop passes on the render actions (we don't process messages directed at this entity)
            while let Some(drawing_actions) = canvas_stream.next().await {
                let maybe_err = drawing_channel.send(DrawingWindowRequest::Draw(DrawingRequest::Draw(Arc::new(drawing_actions)))).await;

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
        let start_time  = Instant::now();

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

                                if Instant::now().duration_since(start_time) >= MAX_BATCH_TIME {
                                    break;
                                }
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
    TDrawStream:    Unpin + Stream<Item=Vec<Draw>>,
    TEventStream:   Unpin + Stream<Item=Vec<DrawEvent>>,
{
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
