use super::render_source_trait::*;
use super::edgeplan_region_renderer::*;
use super::scanline_renderer::*;

use crate::edgeplan::*;
use crate::pixel::*;
use crate::scanplan::*;

impl<TEdge, TScanPlanner, TProgramRunner> RenderSource<TScanPlanner, TProgramRunner> for EdgePlan<TEdge>
where
    TEdge:                  EdgeDescriptor,
    TScanPlanner:           ScanPlanner<Edge=TEdge>,
    TProgramRunner:         PixelProgramRunner,
    TProgramRunner::TPixel: 'static + Send + Copy + AlphaBlend,
{
    /// The region renderer takes instances of this type and uses them to generate pixel values in a region
    type RegionRenderer = EdgePlanRegionRenderer<TEdge, TScanPlanner, ScanlineRenderer<TProgramRunner>>;

    ///
    /// Builds a region renderer that can read from this type and output pixels along rows
    ///
    fn create_region_renderer(planner: TScanPlanner, pixel_runner: TProgramRunner) -> Self::RegionRenderer {
        let scanline_renderer   = ScanlineRenderer::new(pixel_runner);
        let region_renderer     = EdgePlanRegionRenderer::new(planner, scanline_renderer);

        region_renderer
    }
}
