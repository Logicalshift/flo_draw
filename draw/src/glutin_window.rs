use super::draw_event::*;

use flo_stream::*;
use flo_render::*;

use glutin::{WindowedContext, NotCurrent};
use futures::prelude::*;
use gl;

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
pub (super) async fn send_actions_to_window(window: GlutinWindow, render_actions: Subscriber<Vec<RenderAction>>, events: Publisher<DrawEvent>) {
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
