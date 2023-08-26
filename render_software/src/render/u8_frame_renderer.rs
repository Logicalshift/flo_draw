use super::renderer::*;
use super::render_slice::*;
use super::frame_size::*;

use crate::pixel::*;

use std::marker::{PhantomData};

///
/// Renders a whole frame of pixels to a RGBA U8 buffer (using TPixel as the intermediate format)
///
pub struct U8FrameRenderer<TPixel, TRegionRenderer>
where
    TPixel:             Sized + Send + Default + ToGammaColorSpace<U8RgbaPremultipliedPixel>,
    TRegionRenderer:    Renderer<Region=RenderSlice, Dest=[TPixel]>,
{
    region_renderer:    TRegionRenderer,
    pixel:              PhantomData<TPixel>,
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

        // Cut the destination into chunks to form the lines
        let mut chunks  = dest.chunks_exact_mut(region.width).collect::<Vec<_>>();
        let renderer    = &self.region_renderer;

        // Rendering fails if there are insufficient lines to complete
        if chunks.len() < region.height {
            panic!("Cannot render: needed an output buffer large enough to fit {} lines but found {} lines", region.height, chunks.len());
        }

        // Render in chunks of LINES_AT_ONCE lines
        let mut y_idx           = 0;
        let mut render_slice    = RenderSlice { width: region.width, y_positions: vec![] };
        let mut buffer          = vec![TPixel::default(); region.width*LINES_AT_ONCE];
        loop {
            // Stop once we reach the end
            if y_idx >= region.height {
                break;
            }

            // Work out which lines to render next
            let start_idx   = y_idx;
            let end_idx     = start_idx + LINES_AT_ONCE;
            let end_idx     = if end_idx > region.height { region.height } else { end_idx };

            // Write the y positions
            render_slice.y_positions.clear();
            render_slice.y_positions.extend((start_idx..end_idx).map(|idx| idx as f64));

            // Render these lines
            renderer.render(&render_slice, source, &mut buffer);

            // Convert to the final pixel format
            for y_idx in 0..(end_idx-start_idx) {
                let rendered_pixels = &buffer[(y_idx*region.width)..((y_idx+1)*region.width)];
                let target_pixels   = &mut chunks[start_idx + y_idx];

                for x_idx in 0..region.width {
                    target_pixels[x_idx] = rendered_pixels[x_idx].to_gamma_colorspace(region.gamma);
                }
            }

            // Advance to the next y position
            y_idx = end_idx;
        }
    } 
}
