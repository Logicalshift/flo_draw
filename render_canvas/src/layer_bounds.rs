use crate::render_entity_details::RenderEntityDetails;

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
    /// Combines the bounds of the specified entity into this layer
    ///
    pub fn add_entity_with_details(&mut self, details: RenderEntityDetails) {
        self.min_x = f32::min(self.min_x, details.min.0);
        self.min_y = f32::min(self.min_y, details.min.1);
        self.max_x = f32::max(self.max_x, details.max.0);
        self.max_y = f32::max(self.max_y, details.max.1);
    }
}
