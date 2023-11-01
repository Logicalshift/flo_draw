use crate::edgeplan::*;

use flo_canvas as canvas;
use smallvec::*;

use std::ops::{Range};
use std::sync::*;

///
/// Describes the edges of an axis-aligned rectangular region (this is the simplest possible drawing primitive)
///
#[derive(Clone)]
pub struct RectangleEdge {
    shape_id: ShapeId,
    x_bounds: Range<f64>,
    y_bounds: Range<f64>,
}

impl RectangleEdge {
    ///
    /// Creates a new rectangle covering the specified region
    ///
    pub fn new(shape_id: ShapeId, x_bounds: Range<f64>, y_bounds: Range<f64>) -> Self {
        Self { shape_id, x_bounds, y_bounds }
    }
}

impl EdgeDescriptor for RectangleEdge {
    fn clone_as_object(&self) -> Arc<dyn EdgeDescriptor> {
        Arc::new(self.clone())
    }

    #[inline]
    fn prepare_to_render(&mut self) {
    }

    #[inline]
    fn shape(&self) -> ShapeId {
        self.shape_id
    }

    #[inline]
    fn bounding_box(&self) -> ((f64, f64), (f64, f64)) {
        ((self.x_bounds.start, self.y_bounds.start), (self.x_bounds.end, self.y_bounds.end))
    }

    fn transform(&self, transform: &canvas::Transform2D) -> Arc<dyn EdgeDescriptor> {
        todo!()
    }

    #[inline]
    fn intercepts(&self, y_positions: &[f64], output: &mut [SmallVec<[(EdgeInterceptDirection, f64); 2]>]) {
        for idx in 0..y_positions.len() {
            if !(y_positions[idx] < self.y_bounds.start || y_positions[idx] >= self.y_bounds.end) {
                output[idx].push((EdgeInterceptDirection::Toggle, self.x_bounds.start));
                output[idx].push((EdgeInterceptDirection::Toggle, self.x_bounds.end));
            }
        }
    }
}
