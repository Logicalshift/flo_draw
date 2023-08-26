use super::renderer::*;
use super::render_slice::*;

use crate::pixel::*;
use crate::scanplan::*;

///
/// A render source can create an edge region renderer to be used with a render target
///
pub trait RenderSource<'a, TScanPlanner: ScanPlanner, TProgramRunner: PixelProgramRunner<TPixel>, TPixel: 'a> {
    /// The region renderer takes instances of this type and uses them to generate pixel values in a region
    type RegionRenderer: Renderer<Region=RenderSlice, Source=Self, Dest=[&'a mut [TPixel]]>;

    ///
    /// Builds a region renderer that can read from this type and output pixels along rows
    ///
    fn create_region_renderer(planner: &impl ScanPlanner, pixel_runner: &impl PixelProgramRunner<TPixel>) -> Self::RegionRenderer;
}
