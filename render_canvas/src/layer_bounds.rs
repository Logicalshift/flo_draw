use crate::render_entity_details::*;

use flo_canvas as canvas;

///
/// Represents the bounds of a particular layer on the canvas
///
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayerBounds {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

impl Default for LayerBounds {
    fn default() -> Self {
        LayerBounds {
            min_x:  f32::MAX,
            min_y:  f32::MAX,
            max_x:  f32::MIN,
            max_y:  f32::MIN
        }
    }
}

impl LayerBounds {
    ///
    /// True if this represents an 'undefined' bounding box (eg, due to a layer being empty)
    ///
    #[inline]
    pub fn is_undefined(&self) -> bool {
        self.min_x == f32::MAX ||
        self.min_y == f32::MAX ||
        self.max_x == f32::MIN ||
        self.max_y == f32::MIN
    }

    ///
    /// Combines this layer bounds with another layer bounds
    ///
    pub fn combine(&mut self, bounds: &LayerBounds) {
        self.min_x = f32::min(self.min_x, bounds.min_x);
        self.min_y = f32::min(self.min_y, bounds.min_y);
        self.max_x = f32::max(self.max_x, bounds.max_x);
        self.max_y = f32::max(self.max_y, bounds.max_y);
    }

    ///
    /// Combines the bounds of the specified entity into this layer
    ///
    pub fn add_entity_with_details(&mut self, details: RenderEntityDetails) {
        self.min_x = f32::min(self.min_x, details.min.0);
        self.min_y = f32::min(self.min_y, details.min.1);
        self.max_x = f32::max(self.max_x, details.max.0);
        self.max_y = f32::max(self.max_y, details.max.1);
    }

    ///
    /// Returns the effect of transforming these bounds by some transformation
    ///
    pub fn transform(&self, transform: canvas::Transform2D) -> LayerBounds {
        // Transforming has no effect on undefined layer bounds
        if self.is_undefined() { return LayerBounds::default(); }

        // Transform the x and y coordinates of the four corners of the bounding box
        let (x1, y1) = transform.transform_point(self.min_x, self.min_y);
        let (x2, y2) = transform.transform_point(self.max_x, self.max_y);
        let (x3, y3) = transform.transform_point(self.min_x, self.max_y);
        let (x4, y4) = transform.transform_point(self.min_x, self.max_y);

        // Use the min/max values of each coordinate
        LayerBounds {
            min_x: f32::min(f32::min(f32::min(x1, x2), x3), x4),
            min_y: f32::min(f32::min(f32::min(y1, y2), y3), y4),
            max_x: f32::max(f32::max(f32::max(x1, x2), x3), x4),
            max_y: f32::max(f32::max(f32::max(y1, y2), y3), y4),
        }
    }
}
