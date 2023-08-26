#[cfg(feature="term_render")]
mod term_render {
    use super::super::image_render::*;

    ///
    /// Render target that sends its results to the terminal
    ///
    /// (This version only supports the iterm escape sequence)
    ///
    pub struct TermRenderTarget {
        width:  usize,
        height: usize,
    }

    impl TermRenderTarget {
        ///
        /// Creates a terminal rendering target
        ///
        pub fn new(width: usize, height: usize) -> Self {
            TermRenderTarget {
                width, height
            }
        }
    }

    impl<'a, TPixel> RenderTarget<TPixel> for TermRenderTarget
    where
        TPixel:     'static + Send + Copy + Default + AlphaBlend + ToGammaColorSpace<U8RgbaPremultipliedPixel>,
    {
        fn render<TRegionRenderer>(&mut self, region_renderer: TRegionRenderer, source_data: &TRegionRenderer::Source)
        where
            TRegionRenderer: Renderer<Region=RenderSlice, Dest=[TPixel]>
        {
            use base64::engine::general_purpose;

            // Create the png data
            let mut png_data: Vec<u8>   = vec![];
            let mut png_render          = PngRenderTarget::new(&mut png_data, self.width, self.height, 2.2);

            // Render as PNG data
            png_render.render(region_renderer, source_data);

            // TODO: check termial capabilities (we can fall back to an ASCII-art representation)

            // Convert to base64
            let base64 = general_purpose::STANDARD_NO_PAD.encode(&png_data);

            // Write out the iterm escape sequence
            print!("\x1b1337;File=inline:{}\x07", base64);
        }
    }
}

#[cfg(feature="term_render")]
pub use term_render::*;
