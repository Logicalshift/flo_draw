///
/// Specifies the viewport onto the canvas that is rendered into a window
///
/// This is used with a binding set in `FloWindowProperties` to specify the region of the canvas that should be rendered 
/// into a window. By default, `ViewPortBounds::All` is used, which leaves the configuration of the viewport up to the
/// canvas itself.
///
#[derive(Copy, Clone, Debug, PartialEq)]
#[derive(serde::Serialize, serde::Deserialize)]
pub enum ViewportBounds {
    ///
    /// Maps the canvas height to the bounds of the window, using square pixels
    ///
    All,

    ///
    /// Ensures that the canvas height and this specified width are visible
    ///
    Width(f32),

    ///
    /// Ensures that a region, specified as `(minx, miny), (maxx, maxy)`, is visible in the window
    ///
    /// This will scale either to the width or height of the region. Coordinates are in canvas coordinates
    /// at the point that the rendering occurs.
    ///
    /// As for 'All', this tries to ensure that the pixels are square
    ///
    CenterRegion((f32, f32), (f32, f32)),

    ///
    /// Specifies the exact coordinates on the canvas where the corners of the windows should appear
    ///
    /// This will allow the rendering to distort.
    ///
    FitExact((f32, f32), (f32, f32)),
}

impl Default for ViewportBounds {
    #[inline]
    fn default() -> Self {
        ViewportBounds::All
    }
}
