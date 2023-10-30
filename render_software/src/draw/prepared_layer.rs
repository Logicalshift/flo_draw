use crate::edgeplan::*;

use std::sync::*;

///
/// A layer that has been prepared for rendering
///
#[derive(Clone)]
pub struct PreparedLayer {
    /// The edges that are part of this layer (prepared for rendering)
    pub (super) edges: Arc<EdgePlan<Box<dyn EdgeDescriptor>>>,

    /// The bounding box of the edge plan, calculated as it was prepared
    pub (super) bounds: ((f64, f64), (f64, f64)),
}
