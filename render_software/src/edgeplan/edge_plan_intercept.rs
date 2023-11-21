use super::edge_intercept_direction::*;
use super::shape_id::*;

///
/// An intercept found against an edge along a scanline from an edgeplan
///
/// These are all generated against a known y position, so only the x-position of the intercept is specified
///
#[derive(Copy, Clone, Debug)]
pub struct EdgePlanIntercept {
    pub shape:      ShapeId,
    pub direction:  EdgeInterceptDirection,
    pub x_pos:      f64,
}
