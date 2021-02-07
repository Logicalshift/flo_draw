use super::events::*;

use flo_canvas::*;

///
/// Events that can arrive from a flo_draw window
///
#[derive(Clone, PartialEq, Debug)]
pub enum DrawEvent {
    /// Request to re-render the window (this is automatic for canvas windows)
    Redraw,

    /// Indicates that a frame has finished rendering to the canvas
    NewFrame,

    /// The window has a new scale
    Scale(f64),

    /// Window has a new size
    Resize(f64, f64),

    /// Canvas transformation for the window has changed (this will convert between window coordinates and canvas coordinates)
    CanvasTransform(Transform2D),

    /// A pointer device has changed its state
    Pointer(PointerAction, PointerId, PointerState),

    /// Window has been closed
    Closed
}
