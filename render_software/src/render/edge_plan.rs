use super::renderer::*;
use super::edgeplan_region_renderer::*;
use super::scanline_renderer::*;
use super::u8_frame_renderer::*;

use crate::edgeplan::*;
use crate::pixel::*;
use crate::scanplan::PixelScanPlanner;


impl<TEdge> EdgePlan<TEdge>
where
    TEdge: EdgeDescriptor,
{
    ///
    /// Renders an edge plan to an 8-bit RGBA buffer (must contain width*height pixels)
    ///
    pub fn render_whole_frame<TPixel, const N: usize>(&self, program_cache: &PixelProgramCache<TPixel>, data: &PixelProgramDataCache<TPixel>, width: usize, height: usize, gamma: f64, target: &mut [U8RgbaPremultipliedPixel])
    where
        TPixel: 'static + Default + Send + Pixel<N>,
    {
        // TODO:
        //      * Add a way to choose the scan planner to use
        //      * Make it so that the width, height and gamma are all inputs to the appropriate stages instead of initialisation things (much more useful for streaming requests if they can vary)
        //      * Add a trait to make the frame renderer from a target type and a source region renderer
        //      * Add a trait for creating the region renderer from a type (eg, EdgePlan in this case) and a scan planner
        //      * Add a trait for running a program (so we can just pass in that instead of the data type)
        //      * Simplified 'to U8 pixel' type so we don't need to make TPixel a Pixel (and remove the 'N' from everything)
        //      * Some way to do away with the `for<'a> &'a ...` constraints on the region planners

        let scanline_renderer       = ScanlineRenderer::new(program_cache, data);
        let scanline_planner        = PixelScanPlanner::<TEdge>::default();
        let edge_region_renderer    = EdgePlanRegionRenderer::<TEdge, _, _>::new(width, scanline_planner, scanline_renderer);
        let frame_renderer          = U8FrameRenderer::<_, _, EdgePlanRegionRenderer<TEdge, _, _>, N>::new(width, height, gamma, edge_region_renderer);

        (&frame_renderer).render(&(), self, target);
    }
}