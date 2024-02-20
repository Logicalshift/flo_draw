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

impl<TTextureReader, TTexture, const N: usize> PixelProgramForFrame for MipMapTextureProgram<TTextureReader, TTexture, N>
where
    TTexture:       Send + Sync,
    TTextureReader: Copy + Pixel<N> + TextureReader<TTexture>,
{
    type Program    = BilinearTextureProgram<TTextureReader, TTexture, N>;
    type FrameData  = TextureData<MipMap<TTexture>>;

    fn program_for_frame(&self, pixel_size: PixelSize, program_data: &Arc<TextureData<MipMap<TTexture>>>) -> (Self::Program, TextureData<TTexture>) {
        todo!()
    }
}
