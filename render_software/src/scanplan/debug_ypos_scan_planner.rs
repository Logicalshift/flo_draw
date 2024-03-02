use super::scan_planner::*;
use super::scanline_transform::*;
use super::scanline_plan::*;
use super::scanspan::*;
use crate::pixel::*;

///
/// Scanplanner that adds a y position colour indicator pixel to the start of each scanline (which is useful for when we want to find out which scanlines have bugs on them)
///
pub struct DebugYposScanPlanner<TScanPlanner>
where
    TScanPlanner: ScanPlanner,
{
    /// The planner that will be used to generate the main scanline data
    planner:    TScanPlanner,

    /// The pixel program to put at the start of each scanline (should be the debug_ypos prorgam)
    debug_ypos_program: PixelProgramDataId,
}

impl<TScanPlanner> DebugYposScanPlanner<TScanPlanner>
where
    TScanPlanner: ScanPlanner,
{
    ///
    /// Creates a new DebugYposScanPlanner. This will modify the output of `planner` so that it has an indicator pixel at the start of each range
    ///
    pub fn new(planner: TScanPlanner, debug_ypos_program: PixelProgramDataId) -> Self {
        DebugYposScanPlanner { planner, debug_ypos_program }
    }
}

impl<TScanPlanner> ScanPlanner for DebugYposScanPlanner<TScanPlanner>
where
    TScanPlanner: ScanPlanner,
{
    type Edge = TScanPlanner::Edge;

    fn plan_scanlines(&self, edge_plan: &crate::edgeplan::EdgePlan<Self::Edge>, transform: &ScanlineTransform, y_positions: &[f64], x_range: std::ops::Range<f64>, scanlines: &mut [(f64, ScanlinePlan)]) {
        // Ask the undelying planner to generate the scanlines
        self.planner.plan_scanlines(edge_plan, transform, y_positions, x_range.clone(), scanlines);

        // Create the debug plan
        let debug_range = x_range.start..(x_range.start + transform.source_x_to_pixels(1.0));
        let debug_plan  = ScanlinePlan::from_ordered_stacks(vec![ScanSpanStack::with_first_span(ScanSpan::opaque(debug_range, self.debug_ypos_program))]);

        // Combine with the debug program
        for (_ypos, scanline) in scanlines.iter_mut() {
            // The debug plan should be rendered over the top of the plan we generated
            let foreground = debug_plan.clone();

            // Merge the foreground on top of the background
            scanline.merge(&foreground, |src, dst, is_opaque| {
                if is_opaque {
                    *src = dst.clone();
                } else {
                    src.extend(dst.clone());
                }
            });
        }
    }
}