///
/// Specifies a portion of a frame to render
///
#[derive(Debug)]
pub struct RenderSlice {
    /// The width in pixels of a scanline
    pub width: usize,

    /// The y-positions that should be rendered to the buffer
    pub y_positions: Vec<f64>,
}
