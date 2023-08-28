use super::renderer::*;
use super::render_slice::*;
use super::frame_size::*;

use crate::pixel::*;

use std::marker::{PhantomData};
use std::sync::*;

///
/// Renders a whole frame of pixels to a RGBA U8 buffer (using TPixel as the intermediate format)
///
pub struct U8FrameRenderer<TPixel, TRegionRenderer>
where
    TPixel:             Sized + Send + Default + ToGammaColorSpace<U8RgbaPremultipliedPixel>,
    TRegionRenderer:    Renderer<Region=RenderSlice, Dest=[TPixel]>,
{
    region_renderer:    TRegionRenderer,
    pixel:              PhantomData<Mutex<TPixel>>,
}


impl<TPixel, TRegionRenderer> U8FrameRenderer<TPixel, TRegionRenderer>
where
    TPixel:             Sized + Send + Clone + Default + ToGammaColorSpace<U8RgbaPremultipliedPixel>,
    TRegionRenderer:    Renderer<Region=RenderSlice, Dest=[TPixel]>,
{
    ///
    /// Creates a new frame renderer
    ///
    /// Use a gamma value of 2.2 for most rendering tasks (this is the default used by most operating systems)
    ///
    pub fn new(region_renderer: TRegionRenderer) -> Self {
        Self {
            region_renderer:    region_renderer,
            pixel:              PhantomData,
        }
    }
}

impl<'a, TPixel, TRegionRenderer> Renderer for U8FrameRenderer<TPixel, TRegionRenderer> 
where
    TPixel:             Sized + Send + Clone + Default + ToGammaColorSpace<U8RgbaPremultipliedPixel>,
    TRegionRenderer:    Renderer<Region=RenderSlice, Dest=[TPixel]>,
{
    type Region = GammaFrameSize;
    type Source = TRegionRenderer::Source;
    type Dest   = [U8RgbaPremultipliedPixel];

    fn render(&self, region: &GammaFrameSize, source: &TRegionRenderer::Source, dest: &mut [U8RgbaPremultipliedPixel]) {
        const LINES_AT_ONCE: usize = 8;

        // Rendering fails if there are insufficient lines to complete
        if dest.len() < region.width * region.height {
            panic!("Cannot render: needed an output buffer large enough to fit {} lines but found {} lines", region.height, dest.len()/region.width);
        }

        // Cut the destination into chunks to form the lines
        let chunks      = dest.chunks_mut(region.width*LINES_AT_ONCE);
        let renderer    = &self.region_renderer;

        // Render in chunks of LINES_AT_ONCE lines
        let mut render_slice    = RenderSlice { width: region.width, y_positions: vec![] };
        let mut buffer          = vec![TPixel::default(); region.width*LINES_AT_ONCE];

        chunks.enumerate().map(|(chunk_idx, chunk)| {
            let start_y = chunk_idx * LINES_AT_ONCE;
            let end_y   = if start_y + LINES_AT_ONCE > region.height { region.height } else { start_y + LINES_AT_ONCE };

            (start_y..end_y, chunk_idx, chunk)
        }).for_each(|(y_positions, _chunk_idx, chunk)| {
            // Write the y positions
            render_slice.y_positions.clear();
            render_slice.y_positions.extend(y_positions.map(|idx| idx as f64));

            // Render these lines
            renderer.render(&render_slice, source, &mut buffer);

            // Convert to the final pixel format
            TPixel::to_gamma_colorspace(&buffer, chunk, region.gamma);
        });
    } 
}
