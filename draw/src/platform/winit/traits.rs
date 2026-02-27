use winit;

use std::sync::*;

///
/// Trait used to retrieve values from a window for the OS X platform
///
pub trait PlatformWindow {
    ///
    /// Retrieves the winit window for this platform
    ///
    fn window(&self) -> Option<Arc<winit::window::Window>>;
}
