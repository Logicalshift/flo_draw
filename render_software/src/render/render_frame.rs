use crate::pixel::*;
use crate::scanplan::*;

use super::render_source_trait::*;
use super::render_target_trait::*;

///
/// Renders and entire frame to a render target from a render source
///
pub fn render_frame_with_planner<'a, TScanPlanner, TProgramRunner, TSource, TTarget>(scan_planner: TScanPlanner, program_runner: TProgramRunner, source: &TSource, target: &mut TTarget)
where
    TScanPlanner:           ScanPlanner,
    TProgramRunner:         PixelProgramRunner,
    TProgramRunner::TPixel: 'static,
    TSource:                RenderSource<TScanPlanner, TProgramRunner>,
    TTarget:                RenderTarget<TProgramRunner::TPixel>,
{
    let region_renderer = TSource::create_region_renderer(scan_planner, program_runner);
    target.render(region_renderer, source);
}
