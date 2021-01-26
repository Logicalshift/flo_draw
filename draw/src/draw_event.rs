///
/// Events that can arrive from a flo_draw window
///
#[derive(Clone, PartialEq, Debug)]
pub enum DrawEvent {
    /// Request to re-render the window (this is automatic for canvas windows)
    Redraw,

    /// Window has a new size
    Resize(f64, f64)
}
