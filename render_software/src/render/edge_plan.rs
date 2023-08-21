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
        let scanline_renderer       = ScanlineRenderer::new(program_cache, data);
        let scanline_planner        = PixelScanPlanner::<TEdge>::default();
        let edge_region_renderer    = EdgePlanRegionRenderer::<TEdge, _, _>::new(width, scanline_planner, scanline_renderer);
        let frame_renderer          = U8FrameRenderer::<_, _, EdgePlanRegionRenderer<TEdge, _, _>, N>::new(width, height, gamma, edge_region_renderer);

        (&frame_renderer).render(&(), self, target);
    }
}