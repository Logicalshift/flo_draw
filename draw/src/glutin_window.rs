use flo_stream::*;
use flo_render::*;

use glutin::{WindowedContext, NotCurrent};
use futures::prelude::*;

///
/// Manages the state of a Glutin window
///
pub struct GlutinWindow {
    /// The context for this window
    context: WindowedContext<NotCurrent>
}

impl GlutinWindow {
    ///
    /// Creates a new glutin window
    ///
    pub fn new(context: WindowedContext<NotCurrent>) -> GlutinWindow {
        GlutinWindow {
            context:    context
        }
    }
}

///
/// Sends render actions to a window
///
pub (super) async fn send_actions_to_window(window: GlutinWindow, render_actions: Subscriber<Vec<RenderAction>>) {
    // Read events from the render actions list
    let mut render_actions = render_actions;

    while let Some(render_action) = render_actions.next().await {
        // TODO: Draw them to the window context
    }

    // Window will close once the render actions are finished as we drop it here
}
