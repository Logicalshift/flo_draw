use super::scanline_plan::*;
use super::scanline_intercept::*;

use crate::edgeplan::*;
use crate::pixel_program::*;

use std::ops::{Range};

///
/// Creates a pixel-precise scanline plan from an edge plan at a particular y position
///
/// This plan is created from the edge plan, and pixel-aligned to produce a 'jaggy' pixel-precise version of the plan.
/// Ie, no anti-aliasing of any kind is performed with this scanline plan.
///
pub fn plan_pixel_scanlines<TEdge>(edge_plan: &EdgePlan<TEdge>, y_positions: &[f64], x_range: Range<i32>) -> Vec<ScanlinePlan>
where
    TEdge: EdgeDescriptor,
{
    // Ask the edge plan to compute the intercepts on the current scanline
    let mut ordered_intercepts = vec![vec![]; y_positions.len()];
    edge_plan.intercepts_on_scanlines(y_positions, &mut ordered_intercepts);

    ordered_intercepts.into_iter().map(|ordered_intercepts| {
        let mut ordered_intercepts = ordered_intercepts.into_iter();

        // Initial program/position comes from the earliest intercept position
        let mut current_intercept = if let Some(intercept) = ordered_intercepts.next() { intercept } else { return ScanlinePlan::new(); };

        // Trace programs but don't generate fragments until we get an intercept
        let mut active_shapes = ScanlineInterceptState::new();

        while (current_intercept.2.ceil() as i32) < x_range.start {
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
        let mut last_x          = x_range.start;
        let mut program_stack   = vec![];
        let mut scanplan        = vec![];
        let mut z_floor         = active_shapes.z_floor();

        loop {
            // TODO: if a program range is < 1px, instead of just ignoring it, use a blend program (provides horizontal-only anti-aliasing)
            // TODO: if there are multiple intercepts on the same pixel, we should process them all simultaneously (otherwise we will occasionally start a set of programs one pixel too late)
            // TODO: if the program/shape stack doesn not change due to the new intercept (eg, because the change happens behind a longer opaque span), don't create a new span

            // Generate a stack for the current intercept
            let next_x = current_intercept.2.ceil() as i32;

            // The end of the current range is the 'next_x' coordinate
            let next_x      = if next_x > x_range.end { x_range.end } else { next_x };
            let stack_depth = active_shapes.len();

            // We use the z-index of the current shape to determine if it's in front of or behind the current line
            let (shape_id, direction, x_pos)    = &current_intercept;
            let z_index                         = edge_plan.shape_z_index(*shape_id);
            let shape_descriptor                = edge_plan.shape_descriptor(*shape_id);

            if z_index >= z_floor && next_x != last_x && stack_depth > 0 {
                // Create a program stack between the ranges: all the programs until the first opaque layer
                let x_range         = last_x..next_x;
                let mut is_opaque   = false;

                // We re-use program_stack so we don't have to keep re-allocating a vec as we go
                program_stack.clear();
                for shape in (0..stack_depth).rev() {
                    let shape_id    = active_shapes.get(shape).unwrap();
                    let descriptor  = edge_plan.shape_descriptor(shape_id.shape_id()).unwrap();

                    program_stack.extend(descriptor.programs.iter().map(|program| PixelProgramPlan::Run(*program)));
                    if descriptor.is_opaque {
                        is_opaque = true;
                        break;
                    }
                }

                if !program_stack.is_empty() {
                    // Create the stack for these programs
                    let stack = ScanSpanStack::with_reversed_programs(x_range, is_opaque, &program_stack);

                    // Add the stack to the scanplan
                    scanplan.push(stack);
                }
            }

            // Update the state from the current intercept
            active_shapes.add_intercept(*direction, z_index, *shape_id, *x_pos, shape_descriptor.map(|descriptor| descriptor.is_opaque).unwrap_or(false));
            z_floor = active_shapes.z_floor();

            // Next span will start after the end of this one
            last_x = next_x;

            // Stop when the next_x value gets to the end of the range
            if next_x >= x_range.end {
                break;
            }

            // Get ready to process the next intercept in the stack
            current_intercept = if let Some(next_intercept) = ordered_intercepts.next() { next_intercept } else { break; };
        }

        ScanlinePlan::from_ordered_stacks(scanplan)
    }).collect()
}
