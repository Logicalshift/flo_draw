use super::basic_texture::*;
use super::bilinear_texture::*;
use crate::pixel::*;

use std::marker::{PhantomData};
use std::sync::*;

pub struct MipMapTextureProgram<TTextureReader, TTexture, const N: usize>
where
    TTexture:       Send + Sync,
    TTextureReader: Copy + Pixel<N> + TextureReader<TTexture>,
{
    /// Placeholder for the texture data
    texture: PhantomData<MipMap<Arc<TTexture>>>,

    /// Placeholder for the texture reader type
    texture_reader: PhantomData<TTextureReader>
}

impl<TTextureReader, TTexture, const N: usize> Default for MipMapTextureProgram<TTextureReader, TTexture, N> 
where
    TTexture:       Send + Sync,
    TTextureReader: Copy + Pixel<N> + TextureReader<TTexture>,
{
    fn default() -> Self {
        Self {
            texture:        PhantomData,
            texture_reader: PhantomData,
        }
    }
}

impl<TTextureReader, TTexture, const N: usize> PixelProgramForFrame for MipMapTextureProgram<TTextureReader, TTexture, N>
where
    TTexture:       Send + Sync,
    TTextureReader: Copy + Pixel<N> + TextureReader<TTexture>,
{
    type Program    = BilinearTextureProgram<TTextureReader, TTexture, N>;
    type FrameData  = TextureData<MipMap<Arc<TTexture>>>;

    fn program_for_frame(&self, pixel_size: PixelSize, program_data: &Arc<TextureData<MipMap<Arc<TTexture>>>>) -> (Self::Program, TextureData<TTexture>) {
        // Read the transform from the program_data
        let mipmap                  = &*program_data.texture;
        let [[a, b, c], [d, e, f]]  = program_data.transform;

        // Calculate the transform parameters for the texture (we want to know how far we advance in the texture for every x position)
        let dx = a * pixel_size.0;
        let dy = d * pixel_size.0;

        // Fetch the mip level that corresponds to this level
        let mip_level = mipmap.level_for_pixel_step(dx, dy);

        // Figure out the scaling for the mipmap
        // TODO: use the actual texture size here
        let scale = 1.0 / (2.0f64.powi(mip_level as _));

        // Create texture data for reading from this mip-map level
        let mipmap_texture = TextureData {
            texture:    Arc::clone(mipmap.mip_level(mip_level)),
            transform:  [[a*scale, b*scale, c*scale], [d*scale, e*scale, f*scale]]
        };

        // Result is a bilinear filter and the texture
        (BilinearTextureProgram::default(), mipmap_texture)
    }
}
