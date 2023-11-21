use super::edge_intercept_direction::*;

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
}
