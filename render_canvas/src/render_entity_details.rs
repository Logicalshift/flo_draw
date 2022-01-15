use flo_render::*;

///
/// Provides information about a render entity
///
pub struct RenderEntityDetails {
    /// The minimum coordinates for this entity
    pub min: (f32, f32),

    /// The maximum coordinates for this entity
    pub max: (f32, f32),
}

impl RenderEntityDetails {
    ///
    /// Creates a new details object from a set of vertices
    ///
    pub fn from_vertices<'a>(vertices: impl IntoIterator<Item=&'a Vertex2D>) -> RenderEntityDetails {
        let mut min = (f32::MAX, f32::MAX);
        let mut max = (f32::MIN, f32::MIN);

        for vertex in vertices {
            let [x, y] = vertex.pos;

            min.0 = f32::min(x, min.0);
            min.1 = f32::min(y, min.1);
            max.0 = f32::max(x, max.0);
            max.1 = f32::max(y, max.1);
        }

        RenderEntityDetails {
            min, max
        }
    }
}
