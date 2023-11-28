use super::edge_intercept_direction::*;

use std::cmp::{Ordering};

///
/// Describes a position within an edge descriptor
///
/// These are ordered, and have three parts: the subpath ID, the 'edge ID' for where there are multiple edges and the edge position
/// which can distinguish multiple intercepts along the same edge (it's usually the 't' value for the intercept)
///
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EdgePosition(pub usize, pub usize, pub f64);

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

impl PartialOrd for EdgePosition {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for EdgePosition {

}

impl Ord for EdgePosition {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        if self.0 < other.0 {
            Ordering::Less
        } else if self.0 > other.0 {
            Ordering::Greater
        } else if self.1 < other.1 {
            Ordering::Less
        } else if self.1 > other.1 {
            Ordering::Greater
        } else {
            self.2.total_cmp(&other.2)
        }
    }
}