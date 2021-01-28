use super::draw_event::*;
use super::render_window::*;

use flo_canvas::*;
use flo_stream::*;
use flo_render_canvas::*;

use futures::prelude::*;
use futures::task::{Poll, Context};

use std::pin::*;

///
/// Creates a canvas that will render to a window
///
pub fn create_canvas_window() -> (Canvas, Subscriber<DrawEvent>) {
    // Create the canvas
    let canvas                          = Canvas::new();

    // Create a render window
    let (render_actions, window_events) = create_render_window();

    // Get the stream of drawing instructions (and gather them into batches)
    let canvas_stream                   = canvas.stream();
    let canvas_stream                   = BatchedCanvasStream { canvas_stream: Some(canvas_stream) };

    // Create a canvas renderer
    let renderer                        = CanvasRenderer::new();
    let render_events                   = window_events.clone();

    // Return the events
    (canvas, window_events)
}

///
/// Stream that takes a canvas stream and batches as many draw requests as possible
///
struct BatchedCanvasStream<TStream>
where TStream: Stream<Item=Draw> {
    // Stream of individual draw events
    canvas_stream: Option<TStream>
}

impl<TStream> Stream for BatchedCanvasStream<TStream>
where TStream: Unpin+Stream<Item=Draw> {
    type Item = Vec<Draw>;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context) -> Poll<Option<Vec<Draw>>> {
        match &mut self.canvas_stream {
            None                =>  Poll::Ready(None), 
            Some(canvas_stream) => {
                // Poll the canvas stream until there are no more items to fetch
                let mut batch = vec![];

                loop {
                    // Fill up the batch
                    match canvas_stream.poll_next_unpin(context) {
                        Poll::Ready(None)       => {
                            self.canvas_stream = None;
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

                if batch.len() == 0 && self.canvas_stream.is_none() {
                    // Stream finished with no more items
                    Poll::Ready(None)
                } else if batch.len() == 0 && self.canvas_stream.is_some() {
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
