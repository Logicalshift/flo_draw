#[cfg(feature="render_png")]
mod render_png {
    use super::super::frame_size::*;
    use super::super::renderer::*;
    use super::super::render_slice::*;
    use super::super::render_target_trait::*;
    use super::super::u8_frame_renderer::*;

    use crate::pixel::*;

    use std::io::{Write, BufWriter};

    ///
    /// Render target that outputs a PNG file to a stream
    ///
    pub struct PngRenderTarget<TStream>
    where
        TStream: Write,
    {
        writer: png::Writer<BufWriter<TStream>>,
        width:  usize,
        height: usize,
        gamma:  f64,
    }

    impl<TStream> PngRenderTarget<TStream>
    where
        TStream: Write,
    {
        ///
        /// Creates a PNG writer that will write to a stream
        ///
        pub fn from_stream(target: TStream, width: usize, height: usize, gamma: f64) -> Self {
            Self::from_bufwriter(BufWriter::new(target), width, height, gamma)
        }
        ///
        /// Creates a PNG writer that will write to a bufwriter
        ///
        pub fn from_bufwriter(target: BufWriter<TStream>, width: usize, height: usize, gamma: f64) -> Self {
            let mut target = png::Encoder::new(target, width as u32, height as u32);

            target.set_color(png::ColorType::Rgba);
            target.set_depth(png::BitDepth::Eight);
            target.set_source_gamma(png::ScaledFloat::new((1.0/gamma) as _));

            PngRenderTarget {
                writer: target.write_header().unwrap(),
                width:  width,
                height: height,
                gamma:  gamma,
            }
        }
    }

    impl<'a, TStream, TPixel> RenderTarget<TPixel> for PngRenderTarget<TStream> 
    where
        TStream:    Write,
        TPixel:     'static + Send + Copy + Default + AlphaBlend + ToGammaColorSpace<U8RgbaPremultipliedPixel>,
    {
        fn render<TRegionRenderer>(&mut self, region_renderer: TRegionRenderer, source_data: &TRegionRenderer::Source)
        where
            TRegionRenderer: Renderer<Region=RenderSlice, Dest=[TPixel]>
        {
            // Render to a buffer
            // TODO: need to render to a non-premultiplied RGB format for PNG files
            let renderer        = U8FrameRenderer::new(region_renderer);
            let frame_size      = GammaFrameSize { width: self.width, height: self.height, gamma: self.gamma };
            let mut pixel_data  = vec![0u8; self.width*self.height*4];

            renderer.render(&frame_size, source_data, pixel_data.to_rgba_slice_mut());

            // Send the buffer to the png file
            self.writer.write_image_data(&pixel_data).unwrap();
        }
    }
}

#[cfg(feature="render_png")]
pub use render_png::*;
