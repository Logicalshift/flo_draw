use super::canvas_drawing::*;

use crate::pixel::*;
use crate::render::*;
use crate::scanplan::*;

///
/// A canvas region renderer works on a canvas and a rendering state to produce blocks of pixels from
/// a canvas source.
///
pub struct CanvasRegionRenderer<TScanPlanner>
where
    TScanPlanner: ScanPlanner,
{
    scan_planner: TScanPlanner,
}

/* ... TODO: the pixel runner is part of the canvas drawing: ie, we need a different way of passing it in for the edge plan renderer
impl<'a, TScanPlanner, TPixel, const N: usize> RenderSource<TScanPlanner, PixelProgramDataCache<TPixel>> for CanvasDrawingRenderSource<'a, TPixel, N>
where
    TPixel:         'static + Send + Sync + Pixel<N>,
    TScanPlanner:   ScanPlanner,
{
    /// The region renderer takes instances of this type and uses them to generate pixel values in a region
    type RegionRenderer: Renderer<Region=RenderSlice, Source=Self, Dest=[TProgramRunner::TPixel]>;

    ///
    /// Builds a region renderer that can read from this type and output pixels along rows
    ///
    fn create_region_renderer(planner: TScanPlanner, pixel_runner: TProgramRunner) -> Self::RegionRenderer {

    }
}
*/
