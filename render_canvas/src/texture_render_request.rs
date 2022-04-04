use super::layer_handle::*;

use flo_render as render;
use flo_canvas as canvas;

///
/// Textures with pending render instructions
///
#[derive(Clone, Copy)]
pub enum TextureRenderRequest {
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
}
