use super::renderer::*;
use super::render_slice::*;

///
/// Trait implemented by types that can act as a render target
///
/// The 'IntermediatePixel' type is used to perform the initial rendering and blending, before conversion to the final format
///
pub trait RenderTarget<IntermediatePixel: 'static> {
    ///
    /// Renders a frame to this render target
    ///
    /// The renderer that is passed in here is a region renderer, which takes a list of y-positions and generates the pixels for those rows in the results.
    ///
    fn render<TRegionRenderer>(&mut self, region_renderer: TRegionRenderer, source_data: &TRegionRenderer::Source)
    where
        TRegionRenderer: Renderer<Region=RenderSlice, Dest=[IntermediatePixel]>;
}
