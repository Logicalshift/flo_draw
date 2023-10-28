use crate::pixel::*;
use crate::scanplan::*;

use flo_canvas as canvas;

use std::marker::{PhantomData};
use std::ops::{Range};
use std::sync::*;

pub struct TextureData<TTexture>
where
    TTexture:       Send + Sync,
{
    /// The texture that this program will read from
    texture: Arc<TTexture>,

    // The top two rows of the transformation matrix between source coordinates and texture coordinates
    transform: [[f64; 3]; 2],
}

///
/// Program that pixels from a texture, without doing any filtering or mipmapping
///
pub struct BasicTextureProgram<TTextureReader, TTexture>
where
    TTexture:       Send + Sync,
    TTextureReader: TextureReader<TTexture>,
{
    /// Placeholder for the texture data
    texture: PhantomData<Arc<TTexture>>,

    /// Placeholder for the texture reader type
    texture_reader: PhantomData<TTextureReader>
}

impl<TTextureReader, TTexture> Default for BasicTextureProgram<TTextureReader, TTexture>
where
    TTexture:       Send + Sync,
    TTextureReader: TextureReader<TTexture>,
{
    ///
    /// Creates a basic texture program that will read from the specified texture
    ///
    fn default() -> BasicTextureProgram<TTextureReader, TTexture> {
        BasicTextureProgram {
            texture:        PhantomData,
            texture_reader: PhantomData,
        }
    }
}

impl<TTexture> TextureData<TTexture>
where
    TTexture:       Send + Sync,
{
    ///
    /// Creates texture data from a texture and the transform to use
    ///
    pub fn with_texture(texture: Arc<TTexture>, transform: &canvas::Transform2D) -> Self {
        let [[a, b, c], [d, e, f], [_, _, _]] = transform.0;

        TextureData { 
            texture:    texture, 
            transform:  [[a as f64, b as _, c as _], [d as _, e as _, f as _]],
        }
    }
}

impl<TTextureReader, TTexture> PixelProgram for BasicTextureProgram<TTextureReader, TTexture>
where
    TTexture:       Send + Sync,
    TTextureReader: TextureReader<TTexture> + Copy + AlphaBlend,
{
    type Pixel          = TTextureReader;
    type ProgramData    = TextureData<TTexture>;

    #[inline]
    fn draw_pixels(&self, _data_cache: &PixelProgramRenderCache<Self::Pixel>, target: &mut [Self::Pixel], pixel_range: Range<i32>, x_transform: &ScanlineTransform, y_pos: f64, data: &Self::ProgramData) {
        // Read the data
        let texture                 = &*data.texture;
        let [[a, b, c], [d, e, f]]  = data.transform;

        // Convert the start x position to source pixels
        let x_pos   = x_transform.pixel_x_to_source_x(pixel_range.start);

        // Partially calculate the transform and get the pixel size
        let byc     = b * y_pos + c;
        let eyf     = e * y_pos + f;
        let dx      = x_transform.pixel_size();

        // Calculate the position of teh 
        let mut x_pos = x_pos;
        for pixel in target[(pixel_range.start as usize)..(pixel_range.end as usize)].iter_mut() {
            // Calculate the texture position
            let tx = a * x_pos + byc;
            let ty = d * x_pos + eyf;

            *pixel = TTextureReader::read_pixel(texture, tx, ty).source_over(*pixel);

            // Move the x position along
            x_pos += dx;
        }
    }
}
