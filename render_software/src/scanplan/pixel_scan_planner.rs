use super::scanline_intercept::*;
use super::scanline_transform::*;
use super::scanline_plan::*;
use super::scan_planner::*;

use crate::edgeplan::*;
use crate::pixel::*;

use std::marker::{PhantomData};
use std::ops::{Range};
use std::sync::*;

///
/// The pixel scan planner is a basic scan planner that performs no anti-aliasing, so it will produce a 'jaggy' drawing.
///
pub struct PixelScanPlanner<TEdge> {
    edge: PhantomData<Mutex<TEdge>>
}

impl<TEdge> PixelScanPlanner<TEdge>
where
    TEdge: EdgeDescriptor,
{
    ///
    /// Plans out a scanline using the PixelScanPlanner (this scan planner does not perform any anti-aliasing)
    ///
    #[inline]
    pub fn plan(edge_plan: &EdgePlan<TEdge>, transform: &ScanlineTransform, y_positions: &[f64], x_range: Range<f64>) -> Vec<(f64, ScanlinePlan)> {
        // Create a planner and the result vec
        let planner         = Self::default();
        let mut scanlines   = vec![(0.0, ScanlinePlan::default()); y_positions.len()];

        // Fill with scanlines
        planner.plan_scanlines(edge_plan, transform, y_positions, x_range, &mut scanlines);

        scanlines
    }
} 

impl<TEdge> Default for PixelScanPlanner<TEdge>
where
    TEdge: EdgeDescriptor,
{
    #[inline]
    fn default() -> Self {
        PixelScanPlanner { edge: PhantomData }
    }
}

impl<TEdge> ScanPlanner for PixelScanPlanner<TEdge>
where
    TEdge: EdgeDescriptor,
{
    type Edge = TEdge;

    fn plan_scanlines(&self, edge_plan: &EdgePlan<Self::Edge>, transform: &ScanlineTransform, y_positions: &[f64], x_range: Range<f64>, scanlines: &mut [(f64, ScanlinePlan)]) {
        // Must be enough scanlines supplied for filling the scanline array
        if scanlines.len() < y_positions.len() {
            panic!("The number of scanline suppled ({}) is less than the number of y positions to fill them ({})", scanlines.len(), y_positions.len());
        }

        // Map the x-range from the source coordinates to pixel coordinates
        let x_range = transform.source_x_to_pixels(x_range.start)..transform.source_x_to_pixels(x_range.end);

        // Ask the edge plan to compute the intercepts on the current scanline
        let mut ordered_intercepts = vec![vec![]; y_positions.len()];
        edge_plan.intercepts_on_scanlines(y_positions, &mut ordered_intercepts);

        'next_line: for y_idx in 0..y_positions.len() {
            // Fetch/clear the scanline that we'll be building
            let (scanline_pos, scanline) = &mut scanlines[y_idx];
            scanline.clear();
            *scanline_pos = y_positions[y_idx];

            // Iterate over the intercepts on this line
            let ordered_intercepts      = &ordered_intercepts[y_idx];
            let mut ordered_intercepts  = ordered_intercepts.into_iter();

            // Initial program/position comes from the earliest intercept position
            let mut current_intercept = if let Some(intercept) = ordered_intercepts.next() { intercept } else { continue; };

            // Trace programs but don't generate fragments until we get an intercept
            let mut active_shapes = ScanlineInterceptState::new();

            while transform.source_x_to_pixels(current_intercept.x_pos) < x_range.start {
                // Add or remove this intercept's programs to the active list
                let shape_descriptor = edge_plan.shape_descriptor(current_intercept.shape);

                active_shapes.add_intercept(&current_intercept, transform, shape_descriptor);

                // Move to the next intercept (or stop if no intercepts actually fall within the x-range)
                current_intercept = if let Some(intercept) = ordered_intercepts.next() { intercept } else { continue 'next_line; };
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

                // Generate a stack for the current intercept
                let next_x = transform.source_x_to_pixels(current_intercept.x_pos);

                // The end of the current range is the 'next_x' coordinate
                let next_x      = if next_x > x_range.end { x_range.end } else { next_x };
                let stack_depth = active_shapes.len();

                // We use the z-index of the current shape to determine if it's in front of or behind the current line
                let z_index                         = edge_plan.shape_z_index(current_intercept.shape);
                let shape_descriptor                = edge_plan.shape_descriptor(current_intercept.shape);

                if z_index >= z_floor && next_x != last_x {
                    // Create a program stack between the ranges: all the programs until the first opaque layer
                    let x_range         = last_x..next_x;
                    let mut is_opaque   = false;

                    // We re-use program_stack so we don't have to keep re-allocating a vec as we go
                    program_stack.clear();
                    for shape in (0..stack_depth).rev() {
                        let intercept           = active_shapes.get(shape).unwrap();
                        let shape_descriptor    = intercept.shape_descriptor();

                        program_stack.extend(shape_descriptor.programs.iter().map(|program| PixelProgramPlan::Run(*program)));

                        if intercept.is_opaque() {
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

                    // Next span will start after the end of this one
                    last_x = next_x;
                }

                // Update the state from the current intercept
                active_shapes.add_intercept(&current_intercept, transform, shape_descriptor);
                z_floor = active_shapes.z_floor();

                // Stop when the next_x value gets to the end of the range
                if next_x >= x_range.end {
                    break;
                }

                // Get ready to process the next intercept in the stack
                current_intercept = if let Some(next_intercept) = ordered_intercepts.next() { next_intercept } else { break; };
            }

            // Populate the scanline
            #[cfg(debug_assertions)]
            {
                scanline.fill_from_ordered_stacks(scanplan);
            }

            #[cfg(not(debug_assertions))]
            {
                unsafe { scanline.fill_from_ordered_stacks_prechecked(scanplan); }
            }
        }
    }
}
