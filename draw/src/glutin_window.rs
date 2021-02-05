use super::draw_event::*;
use super::window_properties::*;

use flo_stream::*;
use flo_render::*;

use glutin::{WindowedContext, NotCurrent};
use futures::prelude::*;
use futures::task::{Poll, Context};
use gl;

use std::pin::*;

///
/// Manages the state of a Glutin window
///
pub struct GlutinWindow {
    /// The context for this window
    context: Option<WindowedContext<NotCurrent>>,

    /// The renderer for this window (or none if there isn't one yet)
    renderer: Option<GlRenderer>
}

impl GlutinWindow {
    ///
    /// Creates a new glutin window
    ///
    pub fn new(context: WindowedContext<NotCurrent>) -> GlutinWindow {
        GlutinWindow {
            context:    Some(context),
            renderer:   None
        }
    }
}

///
/// Sends render actions to a window
///
pub (super) async fn send_actions_to_window(window: GlutinWindow, render_actions: Subscriber<Vec<RenderAction>>, events: Publisher<DrawEvent>, window_properties: WindowProperties) {
    // Read events from the render actions list
    let mut render_actions  = render_actions;
    let mut window          = window;
    let mut events          = events;

    while let Some(next_action) = render_actions.next().await {
        // Do nothing if there are no actions
        if next_action.len() == 0 {
            continue;
        }

        unsafe {
            // TODO: report errors if we can't set the context rather than just stopping mysteriously

            // Make the current context current
            let current_context = window.context.take().expect("Window context");
            let current_context = current_context.make_current();
            let current_context = if let Ok(context) = current_context { context } else { break; };

            // Get informtion about the current context
            let size            = current_context.window().inner_size();
            let width           = size.width as usize;
            let height          = size.height as usize;

            // Create the renderer (needs the OpenGL functions to be loaded)
            if window.renderer.is_none() {
                // Load the functions for the current context
                // TODO: we're assuming they stay loaded to avoid loading them for every render, which might not be safe
                // TODO: probably better to have the renderer load the functions itself (gl::load doesn't work well
                // when we load GL twice, which could happen if we want to use the offscreen renderer)
                gl::load_with(|symbol_name| {
                    current_context.get_proc_address(symbol_name)
                });

                // Create the renderer
                window.renderer = Some(GlRenderer::new());
            }

            // Perform the rendering actions
            window.renderer.as_mut().map(move |renderer| {
                renderer.prepare_to_render_to_active_framebuffer(width, height);
                renderer.render(next_action);
            });

            // Swap buffers to finish the drawing (TODO: only if ShowFrameBuffer was in the rendering instructions)
            current_context.swap_buffers().ok();

            // Release the current context
            let context     = current_context.make_not_current();
            let context     = if let Ok(context) = context { context } else { break; };
            window.context  = Some(context);

            // Notify that a new frame has been drawn
            events.publish(DrawEvent::NewFrame).await;
        }
    }

    // Window will close once the render actions are finished as we drop it here
}

///
/// The list of update events that can occur to a window
///
enum WindowUpdate {
    Render(Vec<RenderAction>),
    SetTitle(String),
    SetSize((u64, u64)),
    SetFullscreen(bool),
    SetHasDecorations(bool)
}

///
/// Stream that merges the streams from the window properties and the renderer into a single stream
///
struct WindowUpdateStream<TRenderStream, TTitleStream, TSizeStream, TFullscreenStream, TDecorationStream> {
    render_stream:      TRenderStream,
    title_stream:       TTitleStream,
    size:               TSizeStream,
    fullscreen:         TFullscreenStream,
    has_decorations:    TDecorationStream
}

impl<TRenderStream, TTitleStream, TSizeStream, TFullscreenStream, TDecorationStream> Stream for WindowUpdateStream<TRenderStream, TTitleStream, TSizeStream, TFullscreenStream, TDecorationStream>
where
TRenderStream:      Unpin+Stream<Item=Vec<RenderAction>>,
TTitleStream:       Unpin+Stream<Item=String>,
TSizeStream:        Unpin+Stream<Item=(u64, u64)>,
TFullscreenStream:  Unpin+Stream<Item=bool>,
TDecorationStream:  Unpin+Stream<Item=bool> {
    type Item = WindowUpdate;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Poll each stream in turn to see if they have an item

        // Rendering instructions have priority
        match self.render_stream.poll_next_unpin(context) {
            Poll::Ready(Some(item)) => { return Poll::Ready(Some(WindowUpdate::Render(item))); }
            Poll::Ready(None)       => { return Poll::Ready(None); }
            Poll::Pending           => { }
        }

        // The various binding streams
        match self.title_stream.poll_next_unpin(context) {
            Poll::Ready(Some(item)) => { return Poll::Ready(Some(WindowUpdate::SetTitle(item))); }
            Poll::Ready(None)       => { return Poll::Ready(None); }
            Poll::Pending           => { }
        }

        match self.size.poll_next_unpin(context) {
            Poll::Ready(Some(item)) => { return Poll::Ready(Some(WindowUpdate::SetSize(item))); }
            Poll::Ready(None)       => { return Poll::Ready(None); }
            Poll::Pending           => { }
        }

        match self.fullscreen.poll_next_unpin(context) {
            Poll::Ready(Some(item)) => { return Poll::Ready(Some(WindowUpdate::SetFullscreen(item))); }
            Poll::Ready(None)       => { return Poll::Ready(None); }
            Poll::Pending           => { }
        }

        match self.has_decorations.poll_next_unpin(context) {
            Poll::Ready(Some(item)) => { return Poll::Ready(Some(WindowUpdate::SetHasDecorations(item))); }
            Poll::Ready(None)       => { return Poll::Ready(None); }
            Poll::Pending           => { }
        }

        // No stream matched anything
        Poll::Pending
    }
}
