use super::renderer::*;
use super::render_slice::*;
use super::frame_size::*;

use crate::pixel::*;

use std::marker::{PhantomData};
use std::sync::*;

///
/// Renders a whole frame of pixels to a RGBA U16 buffer (using TPixel as the intermediate format)
///
pub struct U16LinearFrameRenderer<TPixel, TRegionRenderer>
where
    TPixel:             Sized + Send + Copy + Clone + Default + ToLinearColorSpace<U16LinearPixel>,
    TRegionRenderer:    Renderer<Region=RenderSlice, Dest=[TPixel]>,
{
    region_renderer:    TRegionRenderer,
    pixel:              PhantomData<Mutex<TPixel>>,
}


impl<TPixel, TRegionRenderer> U16LinearFrameRenderer<TPixel, TRegionRenderer>
where
    TPixel:                 Sized + Send + Copy + Clone + Default + ToLinearColorSpace<U16LinearPixel>,
    TRegionRenderer:        Renderer<Region=RenderSlice, Dest=[TPixel]>,
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

#[cfg(not(feature="multithreading"))]
impl<'a, TPixel, TRegionRenderer> Renderer for U16LinearFrameRenderer<TPixel, TRegionRenderer> 
where
    TPixel:                 Sized + Send + Copy + Clone + Default + ToLinearColorSpace<U16LinearPixel>,
    TRegionRenderer:        Renderer<Region=RenderSlice, Dest=[TPixel]>,
{
    type Region = FrameSize;
    type Source = TRegionRenderer::Source;
    type Dest   = [u16];

    fn render(&self, region: &FrameSize, source: &TRegionRenderer::Source, dest: &mut [u16]) {
        const LINES_AT_ONCE: usize = 8;

        // Rendering fails if there are insufficient lines to complete
        if dest.len() < region.width * region.height * 4 {
            panic!("Cannot render: needed an output buffer large enough to fit {} lines but found {} lines", region.height, dest.len()/(region.width*4));
        }

        // Cut the destination into chunks to form the lines
        let chunks      = dest.chunks_mut(region.width*4*LINES_AT_ONCE);
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
            let target_pixel = U16LinearPixel::u16_slice_as_linear_pixels(chunk);
            TPixel::to_linear_colorspace(&buffer, target_pixel);
        });
    } 
}

#[cfg(feature="multithreading")]
impl<'a, TPixel, TRegionRenderer> Renderer for U16LinearFrameRenderer<TPixel, TRegionRenderer> 
where
    TPixel:                 Sized + Send + Copy + Clone + Default + ToLinearColorSpace<U16LinearPixel>,
    TRegionRenderer:        Renderer<Region=RenderSlice, Dest=[TPixel]>,
{
    type Region = FrameSize;
    type Source = TRegionRenderer::Source;
    type Dest   = [u16];

    fn render(&self, region: &FrameSize, source: &TRegionRenderer::Source, dest: &mut [u16]) {
        const LINES_AT_ONCE: usize = 8;

        use rayon::prelude::*;

        // Rendering fails if there are insufficient lines to complete
        if dest.len() < region.width * region.height * 4 {
            panic!("Cannot render: needed an output buffer large enough to fit {} lines but found {} lines", region.height, dest.len()/(region.width*4));
        }

        // Cut the destination into chunks to form the lines
        let chunks      = dest.par_chunks_mut(region.width*4*LINES_AT_ONCE);
        let renderer    = &self.region_renderer;

        chunks.enumerate().map(|(chunk_idx, chunk)| {
            let start_y = chunk_idx * LINES_AT_ONCE;
            let end_y   = if start_y + LINES_AT_ONCE > region.height { region.height } else { start_y + LINES_AT_ONCE };

            (start_y..end_y, chunk_idx, chunk)
        }).for_each_init(|| {
                let render_slice    = RenderSlice { width: region.width, y_positions: vec![] };
                let buffer          = vec![TPixel::default(); region.width*LINES_AT_ONCE];

                (render_slice, buffer)
            }, 
            |(ref mut render_slice, ref mut buffer), (y_positions, _chunk_idx, chunk)| {
                // Write the y positions
                render_slice.y_positions.clear();
                render_slice.y_positions.extend(y_positions.map(|idx| 58.5 as f64 + ((idx as f64)/256.0)));

                // Render these lines
                renderer.render(&render_slice, source, buffer);

                // Convert to the final pixel format
                let target_pixel = U16LinearPixel::u16_slice_as_linear_pixels(chunk);
                TPixel::to_linear_colorspace(&buffer, target_pixel);
            });
    } 
}
