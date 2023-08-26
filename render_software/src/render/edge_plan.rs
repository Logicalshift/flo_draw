use super::render_source_trait::*;
use super::edgeplan_region_renderer::*;
use super::scanline_renderer::*;

use crate::edgeplan::*;
use crate::pixel::*;
use crate::scanplan::*;

impl<'a, TEdge, TScanPlanner, TProgramRunner, TPixel> RenderSource<'a, TScanPlanner, TProgramRunner, TPixel> for EdgePlan<TEdge>
where
    TEdge:          'a + EdgeDescriptor,
    TPixel:         'static + Send + Copy + AlphaBlend,
    TScanPlanner:   'a + ScanPlanner<Edge=TEdge>,
    TProgramRunner: 'a + PixelProgramRunner<TPixel>,
{
    /// The region renderer takes instances of this type and uses them to generate pixel values in a region
    // TODO: using a reference here (required due to some later borrowing requirements) doesn't work
    type RegionRenderer = EdgePlanRegionRenderer<TEdge, TScanPlanner, ScanlineRenderer<'a, TPixel, TProgramRunner>>;

    ///
    /// Builds a region renderer that can read from this type and output pixels along rows
    ///
    fn create_region_renderer(planner: TScanPlanner, pixel_runner: &'a TProgramRunner) -> Self::RegionRenderer {
        let scanline_renderer   = ScanlineRenderer::new(pixel_runner);
        let region_renderer     = EdgePlanRegionRenderer::<TEdge, TScanPlanner, ScanlineRenderer<'a, TPixel, TProgramRunner>>::new(planner, scanline_renderer);

        region_renderer
    }
}
