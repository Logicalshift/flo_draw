use super::scanline_plan::*;
use crate::edgeplan::*;

use std::ops::{Range};

///
/// A scan planner is an algorithm that discovers where along a scanline to render pixels using pixel programs
///
pub trait ScanPlanner {
    /// The type of edge stored in the edge plan for this planner
    type Edge: EdgeDescriptor;

    /// For every scanline in `y_positions`, use the edge plan to find the intercepts at a set of y-positions, clipped to the specified x-range, and
    /// generating the output in the `scanlines` array
    fn plan_scanlines(edge_plan: &EdgePlan<Self::Edge>, y_positions: &[f64], x_range: Range<f64>, scanlines: &mut [ScanlinePlan]);
}
