use flo_canvas as canvas;
use flo_render as render;

///
/// The settings for a path
///
#[derive(Clone, Debug)]
pub struct StrokeSettings {
    pub stroke_color:   render::Rgba8,
    pub join:           canvas::LineJoin,
    pub cap:            canvas::LineCap,
    pub dash_pattern:   Vec<f32>,
    pub dash_offset:    f32,
    pub line_width:     f32
}

impl StrokeSettings {
    ///
    /// Creates a new path settings with the default values for the renderer
    ///
    pub fn new() -> StrokeSettings {
        StrokeSettings {
            stroke_color:   render::Rgba8([0, 0, 0, 255]),
            join:           canvas::LineJoin::Round,
            cap:            canvas::LineCap::Butt,
            dash_pattern:   vec![],
            dash_offset:    0.0,
            line_width:     1.0
        }
    }
}
