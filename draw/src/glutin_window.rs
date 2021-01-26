use glutin::{WindowedContext, NotCurrent};

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