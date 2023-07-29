use super::edge_descriptor::*;
use super::edge_plan::*;
use super::scanline_intercept::*;

use crate::scanplan::*;

use std::ops::{Range};

///
/// Creates a pixel-precise scanline plan from an edge plan at a particular y position
///
/// This plan is created from the edge plan, and pixel-aligned to produce a 'jaggy' pixel-precise version of the plan.
/// Ie, no anti-aliasing of any kind is performed with this scanline plan.
///
pub fn plan_pixel_scanline<TEdge>(edge_plan: &EdgePlan<TEdge>, y_pos: f64, x_range: Range<i64>) -> ScanlinePlan 
where
    TEdge: EdgeDescriptor,
{
    // Ask the edge plan to compute the intercepts on the current scanline
    let mut ordered_intercepts = edge_plan.intercepts_on_scanline(y_pos);

    // Initial program/position comes from the earliest intercept position
    let mut current_intercept = if let Some(intercept) = ordered_intercepts.next() { intercept } else { return ScanlinePlan::new(); };

    // Trace programs but don't generate fragments until we get an intercept
    let mut active_shapes = ScanlineInterceptState::new();

    while (current_intercept.2.ceil() as i64) < x_range.start {
        // Add or remove this intercept's programs to the active list
        let (shape_id, direction, x_pos)    = &current_intercept;
        let z_index                         = edge_plan.shape_z_index(*shape_id);

        active_shapes.add_intercept(*direction, z_index, *shape_id, *x_pos);

        // Move to the next intercept (or stop if no intercepts actually fall within the x-range)
        if let Some(intercept) = ordered_intercepts.next() { intercept } else { return ScanlinePlan::new(); };
    }

    // Update all of the existing shapes to have a start position at the left-hand side of the screen
    active_shapes.clip_start_x(x_range.start as _);

    // Read intercepts until we reach the x_range end, and generate the program stacks for the scanline plan

    // If there's still a final intercept, then generate a final scan region to the end of the x_range

    todo!()
}
