use super::layer_handle::*;

use flo_render as render;
use flo_canvas as canvas;

use std::sync::*;

///
/// Requests to render vertex data to textures
///
/// These actions are taken after layer tessellation has completed but before any other rendering instructions (including the setup instructions)
///
#[derive(Clone, Debug)]
pub enum TextureRenderRequest {
    ///
    /// Creates an empty texture of a particular size
    ///
    CreateBlankTexture(render::TextureId, canvas::TextureSize, canvas::TextureFormat),

    ///
    /// Changes the bytes representing a rectangular region of this texture
    ///
    SetBytes(render::TextureId, canvas::TexturePosition, canvas::TextureSize, Arc<Vec<u8>>),

    ///
    /// Apply mipmaps to the specified texture
    ///
    CreateMipMaps(render::TextureId),

    ///
    /// The specified sprite bounds should be made to fill the texture
    ///
    /// Once this instruction has been completed by a stream, the texture will not be rendered again
    ///
    FromSprite(render::TextureId, LayerHandle, canvas::SpriteBounds),

    ///
    /// A dynamic texture is re-rendered any time the layer or the canvas size changes
    ///
    DynamicTexture(render::TextureId, LayerHandle, canvas::SpriteBounds, canvas::CanvasSize, canvas::Transform2D),

    ///
    /// Copy the first texture to the second texture, then decrease the usage count of the first texture
    ///
    CopyTexture(render::TextureId, render::TextureId),
}
