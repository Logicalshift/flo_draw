use crate::texture::*;

use std::sync::*;

///
/// Canvas texture definition
///
/// Texture data is optional here: we won't be able to reconstruct textures if this is not supplied but
/// also won't duplicate them in the renderer (or spend time copying the bytes into multiple buffers)
///
#[derive(Clone, Debug)]
pub struct CanvasTexture {
    format:             TextureFormat,
    size:               (f32, f32),
    bytes:              CanvasTextureData,
    fill_transparency:  f32,
}

///
/// Data storage for a texture
///
#[derive(Clone, Debug)]
pub enum CanvasTextureData {
    None,
    Modified(Vec<u8>),
    Shared(Arc<Vec<u8>>),
}
