use super::scan_planner::*;
use super::scanline_transform::*;
use super::scanline_plan::*;
use crate::pixel::*;

///
/// Scanplanner that adds a background colour to the output of another scan planner
///
pub struct BackgroundScanPlanner<TScanPlanner>
where
    TScanPlanner: ScanPlanner,
{
    /// The planner that will be used to generate the main scanline data
    planner:    TScanPlanner,

    /// The pixel program to run as the background for the result of this scanplanner
    background: PixelProgramDataId,
}

impl<TScanPlanner> BackgroundScanPlanner<TScanPlanner>
where
    TScanPlanner: ScanPlanner,
{
    ///
    /// Creates a new BackgroundScanPlanner. This will modify the output of `planner` so that it is on top of a background pixel program
    ///
    pub fn new(planner: TScanPlanner, background: PixelProgramDataId) -> Self {
        BackgroundScanPlanner { planner, background }
    }
}

impl<TScanPlanner> ScanPlanner for BackgroundScanPlanner<TScanPlanner>
where
    TScanPlanner: ScanPlanner,
{
    type Edge = TScanPlanner::Edge;

    fn plan_scanlines(&self, edge_plan: &crate::edgeplan::EdgePlan<Self::Edge>, transform: &ScanlineTransform, y_positions: &[f64], pixel_x_range: std::ops::Range<i32>, scanlines: &mut [(f64, ScanlinePlan)]) {
        // Ask the undelying planner to generate the scanlines
        self.planner.plan_scanlines(edge_plan, transform, y_positions, pixel_x_range.clone(), scanlines);

        // Create the background plan
        // TODO: something weird here, the original seemed to be using pixel ranges here (but the shard planner definitely converts from canvas coordinates)...
        // TODO: I think the background plan should be in pixel units, not canvas units
        let mut background_plan = ScanlinePlan::default();
        background_plan.push_next_range(transform.pixel_range_to_x(&pixel_x_range), true, [PixelProgramPlan::Run(self.background)]);

        // Combine with the background program
        for (_ypos, scanline) in scanlines.iter_mut() {
            use std::mem;

            // The content of the scanline is the foreground: swap it with the background
            let mut foreground = background_plan.clone();
            mem::swap(&mut foreground, scanline);

            // Merge the foreground on top of the background
            scanline.merge(&foreground, |src, dst, is_opaque| {
                if is_opaque {
                    src.clear();
                }

                src.extend(dst);
            });
        }
    }
}