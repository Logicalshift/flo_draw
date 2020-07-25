use super::path_settings::*;

use flo_render as render;
use flo_canvas as canvas;

use lyon::path;

///
/// The tesselator turns canvas drawing instructions into triangle lists for rendering
///
pub struct Tessellator {
    /// The builder for the current path
    path_builder: path::Builder,

    /// None, or the built path
    current_path: Option<path::Path>,

    /// The rendered fill for this path
    fill: Option<Vec<render::Vertex2D>>,

    /// The rendered stroke for this path
    stroke: Option<Vec<render::Vertex2D>>,

    /// The path settings for this path
    path_settings: PathSettings
}

impl Tessellator {
    ///
    /// Creates a new path tessellator
    ///
    pub fn new() -> Tessellator {
        Tessellator {
            path_builder:   path::Path::builder(),
            current_path:   None,
            fill:           None,
            stroke:         None,
            path_settings:  PathSettings::new()
        }
    }
}
