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

///
/// An intercept found against an edge along a pair of scanlines from an edgeplan
///
/// These are generated between a pair of y positions
///
#[derive(Copy, Clone, Debug)]
pub struct EdgePlanShardIntercept {
    pub shape:      ShapeId,
    pub direction:  EdgeInterceptDirection,
    pub lower_x:    f64,
    pub upper_x:    f64,
}