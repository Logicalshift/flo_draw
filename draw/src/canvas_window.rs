use super::draw_event::*;
use super::glutin_thread::*;
use super::render_window::*;
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
    renderer:   CanvasRenderer,

    /// Represents the current canvas state
    canvas:     Canvas
}

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
    let renderer                        = RendererState { renderer: renderer, canvas: Canvas::new() };
    let renderer                        = Arc::new(Desync::new(renderer));
    let render_events                   = window_events.clone();

    // Handle events from the window
    let redraw_render_actions           = render_actions.republish();
    pipe_in(Arc::clone(&renderer), render_events, move |state, event| { 
        let mut redraw_render_actions = redraw_render_actions.republish();
        async move { 
            handle_event(state, event, &mut redraw_render_actions).await; 
        }.boxed() 
    });

    // Pipe from the canvas stream to the renderer to generate a stream of render actions
    let render_action_stream            = pipe(Arc::clone(&renderer), canvas_stream, 
        |state, drawing| async move { 
            state.canvas.write(drawing.clone());
            state.renderer.draw(drawing.into_iter()).collect::<Vec<_>>().await
        }.boxed());

    // Await the rendering future on the glutin thread
    glutin_thread().send_event(GlutinThreadEvent::RunProcess(Box::new(move || async move {
        // Publish the resulting actions to glutin
        let mut render_actions          = render_actions;
        let rendering                   = render_actions.send_all(render_action_stream);

        rendering.await;
    }.boxed_local())));

    // Return the events
    (canvas, window_events)
}

///
/// Handles an event from the window
///
fn handle_event<'a>(state: &'a mut RendererState, event: DrawEvent, render_actions: &'a mut Publisher<Vec<RenderAction>>) -> impl 'a+Send+Future<Output=()> {
    async move {
        match event {
            DrawEvent::Redraw                   => { 
                let redraw = state.renderer.draw(state.canvas.get_drawing().into_iter()).collect::<Vec<_>>().await;
                render_actions.publish(redraw).await;
            },

            DrawEvent::Resize(width, height)    => { 
                let (width, height) = (width as f32, height as f32); 

                let scale           = 2.0;
                let vp_x            = 0.0;
                let vp_y            = 0.0;

                state.renderer.set_viewport(vp_x..vp_x+width, vp_y..vp_y+height, width, height, scale); 
            }
        }
    }
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
