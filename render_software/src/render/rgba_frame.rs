use super::frame_size::*;
use super::renderer::*;
use super::render_slice::*;
use super::render_target_trait::*;
use super::u8_frame_renderer::*;

use crate::pixel::*;

///
/// A render target of a frame of u8 pixels with pre-multiplied alpha
///
pub struct RgbaFrame<'a> {
    pub width:      usize,
    pub height:     usize,
    pub gamma:      f64,
    pub pixel_data: &'a mut [U8RgbaPremultipliedPixel],
}

impl<'a> RgbaFrame<'a> {
    ///
    /// Creates a RgbaFrame render target from a buffer of U8RgbaPremultipliedPixel pixels (returns an error if the buffer is not big enough)
    ///
    #[inline]
    pub fn from_pixels(width: usize, height: usize, gamma: f64, data: &'a mut [U8RgbaPremultipliedPixel]) -> Result<Self, ()> {
        if data.len() < width*height {
            Err(())
        } else {
            Ok(RgbaFrame {
                width:      width,
                height:     height,
                gamma:      gamma,
                pixel_data: data,
            })
        }
    }

    ///
    /// Creates a RgbaFrame render target from a buffer of u8 values (which will be rendered as R, G, B, A pixels)
    ///
    #[inline]
    pub fn from_bytes(width: usize, height: usize, gamma: f64, data: &'a mut [u8]) -> Result<Self, ()> {
        Self::from_pixels(width, height, gamma, data.to_rgba_slice_mut())
    }
}

impl<'a, TPixel> RenderTarget<TPixel> for RgbaFrame<'a> 
where
    TPixel: 'static + Send + Copy + Default + AlphaBlend + ToGammaColorSpace<U8RgbaPremultipliedPixel>,
{
    fn render<TRegionRenderer>(&mut self, region_renderer: TRegionRenderer, source_data: &TRegionRenderer::Source)
    where
        TRegionRenderer: Renderer<Region=RenderSlice, Dest=[TPixel]>
    {
        let renderer    = U8FrameRenderer::new(region_renderer);
        let frame_size  = GammaFrameSize { width: self.width, height: self.height, gamma: self.gamma };

        renderer.render(&frame_size, source_data, self.pixel_data)
    }
}
