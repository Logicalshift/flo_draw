use super::basic_texture::*;
use crate::pixel::*;
use crate::scanplan::*;

use std::marker::{PhantomData};
use std::ops::{Range};
use std::sync::*;

///
/// Program that reads pixels from a texture and renders using bilinear filtering only
///
pub struct BilinearTextureProgram<TTextureReader, TTexture, const N: usize>
where
    TTexture:       Send + Sync,
    TTextureReader: Copy + Pixel<N> + TextureReader<TTexture>,
{
    /// Placeholder for the texture data
    texture: PhantomData<Arc<TTexture>>,

    /// Placeholder for the texture reader type
    texture_reader: PhantomData<TTextureReader>
}

impl<TTextureReader, TTexture, const N: usize> Default for BilinearTextureProgram<TTextureReader, TTexture, N>
where
    TTexture:       Send + Sync,
    TTextureReader: Copy + Pixel<N> + TextureReader<TTexture>,
{
    ///
    /// Creates a basic texture program that will read from the specified texture
    ///
    fn default() -> BilinearTextureProgram<TTextureReader, TTexture, N> {
        BilinearTextureProgram {
            texture:        PhantomData,
            texture_reader: PhantomData,
        }
    }
}

impl<TTextureReader, TTexture, const N: usize> PixelProgram for BilinearTextureProgram<TTextureReader, TTexture, N>
where
    TTexture:       Send + Sync,
    TTextureReader: Copy + Pixel<N> + TextureReader<TTexture>,
{
    type Pixel          = TTextureReader;
    type ProgramData    = TextureData<TTexture>;

    #[inline]
    fn draw_pixels(&self, _data_cache: &PixelProgramRenderCache<Self::Pixel>, target: &mut [Self::Pixel], pixel_range: Range<i32>, x_transform: &ScanlineTransform, y_pos: f64, data: &Self::ProgramData) {
        // Read the data
        let texture                 = &*data.texture;
        let alpha                   = data.alpha;
        let [[a, b, c], [d, e, f]]  = data.transform;

        // Convert the start x position to source pixels
        let x_pos   = x_transform.pixel_x_to_source_x(pixel_range.start);

        // Partially calculate the transform and get the pixel size
        let byc     = b * y_pos + c;
        let eyf     = e * y_pos + f;
        let dx      = x_transform.pixel_size();

        // Read from the texture into the pixel range
        let mut texture_pixels = TTextureReader::read_pixels_linear_bilinear_filter(texture, x_pos, dx, (a, byc), (d, eyf), pixel_range.len());

        // Alpha-blend the pixels into the final result
        if alpha >= 1.0 {
            for (texture_pixel, tgt_pixel) in texture_pixels.into_iter().zip((&mut target[(pixel_range.start as usize)..(pixel_range.end as usize)]).iter_mut()) {
                *tgt_pixel = texture_pixel.source_over(*tgt_pixel);
            }
        } else {
            let alpha = TTextureReader::Component::with_value(alpha);

            for (texture_pixel, tgt_pixel) in texture_pixels.into_iter().zip((&mut target[(pixel_range.start as usize)..(pixel_range.end as usize)]).iter_mut()) {
                *tgt_pixel = (texture_pixel * alpha).source_over(*tgt_pixel);
            }
        }
    }
}
