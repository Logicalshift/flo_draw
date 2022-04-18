use crate::render_entity_details::*;

use flo_canvas as canvas;
use flo_render as render;

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

impl Into<render::FrameBufferRegion> for LayerBounds {
    fn into(self) -> render::FrameBufferRegion {
        render::FrameBufferRegion((self.min_x, self.min_y), (self.max_x, self.max_y))
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
    /// Returns the overlapping region between two bounds (or None if the bounds do not overlap)
    ///
    pub fn clip(&self, bounds: &LayerBounds) -> Option<LayerBounds> {
        let new_bounds = LayerBounds {
            min_x: f32::max(self.min_x, bounds.min_x),
            min_y: f32::max(self.min_y, bounds.min_y),
            max_x: f32::min(self.max_x, bounds.max_x),
            max_y: f32::min(self.max_y, bounds.max_y),
        };

        if new_bounds.min_x > new_bounds.max_x || new_bounds.min_y > new_bounds.max_y {
            None
        } else {
            Some(new_bounds)
        }
    }

    ///
    /// Combines the bounds of the specified entity into this layer
    ///
    pub fn add_entity_with_details(&mut self, details: RenderEntityDetails) {
        self.combine(&details.bounds);
    }

    ///
    /// Returns the effect of transforming these bounds by some transformation
    ///
    pub fn transform(&self, transform: &canvas::Transform2D) -> LayerBounds {
        // Transforming has no effect on undefined layer bounds
        if self.is_undefined() { return LayerBounds::default(); }

        // Transform the x and y coordinates of the four corners of the bounding box
        let (x1, y1) = transform.transform_point(self.min_x, self.min_y);
        let (x2, y2) = transform.transform_point(self.max_x, self.min_y);
        let (x3, y3) = transform.transform_point(self.min_x, self.max_y);
        let (x4, y4) = transform.transform_point(self.max_x, self.max_y);

        // Use the min/max values of each coordinate
        LayerBounds {
            min_x: f32::min(f32::min(f32::min(x1, x2), x3), x4),
            min_y: f32::min(f32::min(f32::min(y1, y2), y3), y4),
            max_x: f32::max(f32::max(f32::max(x1, x2), x3), x4),
            max_y: f32::max(f32::max(f32::max(y1, y2), y3), y4),
        }
    }

    ///
    /// Converts the coordinates in these bounds into the number of pixels it will occupy in a viewport of the specified size
    ///
    pub fn to_viewport_pixels(&self, viewport_size: &render::Size2D) -> LayerBounds {
        // The viewport occupies the coordinates -1 to 1: these map to the pixel coordinates 0-viewport_size
        let render::Size2D(w, h)    = viewport_size;
        let w                       = *w as f32;
        let h                       = *h as f32;

        LayerBounds {
            min_x: (self.min_x + 1.0)/2.0 * w,
            min_y: (self.min_y + 1.0)/2.0 * h,
            max_x: (self.max_x + 1.0)/2.0 * w,
            max_y: (self.max_y + 1.0)/2.0 * h,
        }
    }
}
