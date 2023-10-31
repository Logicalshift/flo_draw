use crate::edgeplan::*;

use flo_canvas as canvas;

use std::sync::*;

///
/// A layer that has been prepared for rendering
///
#[derive(Clone)]
pub struct PreparedLayer {
    /// The edges that are part of this layer (prepared for rendering)
    pub (super) edges: Arc<EdgePlan<Arc<dyn EdgeDescriptor>>>,

    /// The bounding box of the edge plan, calculated as it was prepared
    pub (super) bounds: ((f64, f64), (f64, f64)),

    /// The transform to map sprite coordinates to render coordinates (render coordinates are used by the edge plan)
    pub (super) transform: canvas::Transform2D,

    /// Transform to map render coordinates to sprite coordinates (the coordinates used by the original render)
    ///
    /// Note that we store the sprite in render coordinate as things like the flattening edges assume that the coordinates
    /// work this way (and are thus simpler as they don't have to understand the difference between a sprite and a normal layer)
    pub (super) inverse_transform: canvas::Transform2D,
}
