use super::layer_bounds::*;

use flo_render::*;
use flo_canvas as canvas;

///
/// Provides information about a render entity
///
pub struct RenderEntityDetails {
    /// The bounds for the render entity
    pub bounds: LayerBounds
}

impl RenderEntityDetails {
    ///
    /// Creates a new details object from a set of vertices
    ///
    pub fn from_vertices<'a>(vertices: impl IntoIterator<Item=&'a Vertex2D>, transform: &canvas::Transform2D) -> RenderEntityDetails {
        // Work out the minimum and maximu
        let mut min = (f32::MAX, f32::MAX);
        let mut max = (f32::MIN, f32::MIN);

        for vertex in vertices {
            let [x, y] = vertex.pos;

            min.0 = f32::min(x, min.0);
            min.1 = f32::min(y, min.1);
            max.0 = f32::max(x, max.0);
            max.1 = f32::max(y, max.1);
        }

        // Get the bounds and convert via the transform
        let bounds = LayerBounds {
            min_x: min.0,
            min_y: min.1,
            max_x: max.0,
            max_y: max.1
        };
        let bounds = bounds.transform(transform);

        RenderEntityDetails {
            bounds
        }
    }
}
