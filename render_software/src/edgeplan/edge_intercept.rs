use super::edge_descriptor::*;
use super::shape_id::*;

///
/// An intercept found against an edge along a scanline
///
/// These are all generated against a known y position, so only the x-position of the intercept is specified
///
#[derive(Clone, Debug)]
pub struct EdgeIntercept {
    pub shape:      ShapeId,
    pub direction:  EdgeInterceptDirection,
    pub x_pos:      f64,
}
