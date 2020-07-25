use lyon::path;

///
/// The tesselator turns canvas drawing instructions into triangle lists for rendering
///
pub struct Tessellator {
    /// The builder for the current path
    path_builder: path::Builder,

    /// None, or the built path
    current_path: Option<path::Path>,
}

impl Tessellator {
    ///
    /// Creates a new path tessellator
    ///
    pub fn new() -> Tessellator {
        Tessellator {
            path_builder: path::Path::builder(),
            current_path: None
        }
    }
}
