use super::edge_intercept_direction::*;

///
/// Describes a position within an edge descriptor
///
/// These are ordered, and have two parts: the 'edge ID' for where there are multiple edges and the edge position which can 
/// distinguish multiple intercepts along the same edge (it's usually the 't' value for the intercept)
///
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct EdgePosition(pub usize, pub f64);

///
/// Describes an intercept from an edge descriptor
///
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct EdgeDescriptorIntercept {
    ///
    /// The x-position of this intercept
    ///
    pub x_pos: f64,

    ///
    /// The direction that the edge that is being crossed is travelling in
    ///
    pub direction: EdgeInterceptDirection,

    ///
    /// The position of this intercept
    ///
    pub position: EdgePosition,
}
