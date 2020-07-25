use flo_canvas as canvas;
use flo_render as render;

///
/// The settings for a path
///
#[derive(Clone, Debug)]
pub struct PathSettings {
    pub fill_color:     render::Rgba8,
    pub stroke_color:   render::Rgba8,
    pub join:           canvas::LineJoin,
    pub cap:            canvas::LineCap,
    pub dash_pattern:   Vec<f32>,
    pub line_width:     f32
}

impl PathSettings {
    ///
    /// Creates a new path settings with the default values for the renderer
    ///
    pub fn new() -> PathSettings {
        PathSettings {
            fill_color:     render::Rgba8([0, 0, 0, 255]),
            stroke_color:   render::Rgba8([0, 0, 0, 255]),
            join:           canvas::LineJoin::Round,
            cap:            canvas::LineCap::Butt,
            dash_pattern:   vec![],
            line_width:     1.0
        }
    }
}
