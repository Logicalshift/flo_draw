use super::scanline_plan::*;
use crate::edgeplan::*;

use std::ops::{Range};

///
/// A scan planner is an algorithm that discovers where along a scanline to render pixels using pixel programs
///
pub trait ScanPlanner : Send + Sync {
    /// The type of edge stored in the edge plan for this planner
    type Edge: EdgeDescriptor;

    ///
    /// For every scanline in `y_positions`, use the edge plan to find the intercepts at a set of y-positions, clipped to the specified x-range, and
    /// generating the output in the `scanlines` array.
    ///
    /// The y-position is copied into the scanlines array, and the scanlines are always generated in the same order that they are requested in.
    ///
    fn plan_scanlines(&self, edge_plan: &EdgePlan<Self::Edge>, y_positions: &[f64], x_range: Range<f64>, scanlines: &mut [(f64, ScanlinePlan)]);
}
