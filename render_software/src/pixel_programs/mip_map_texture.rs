use super::basic_texture::*;
use super::bilinear_texture::*;
use crate::pixel::*;
use crate::scanplan::*;

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
