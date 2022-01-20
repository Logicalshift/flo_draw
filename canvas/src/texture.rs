use crate::sprite::*;

use std::sync::*;

///
/// The position of a pixel within a texture, in pixels
///
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct TexturePosition(pub u32, pub u32);

///
/// The width and height of a texture, in pixels
///
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct TextureSize(pub u32, pub u32);

///
/// Format of a rendering texture
///
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum TextureFormat {
    /// Every pixel is 4 bytes specifying the red, green, blue and alpha values for the pixel
    Rgba
}

///
/// Size of a region on the canvas
///
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct CanvasSize(pub f32, pub f32);

///
/// Operations that can be performed on a texture
///
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum TextureOp { 
    /// Creates a new texture of the specified size and format (texture colour is set to clear)
    Create(TextureSize, TextureFormat),

    /// Releases the memory used by this texture
    Free,

    /// Sets a region of a texture (specified as minx, miny, width, height) to the specified bitmap
    SetBytes(TexturePosition, TextureSize, Arc<Vec<u8>>),

    /// Renders the specified sprite to the texture (mapping the supplied bounds to the coordinates in the texture)
    SetFromSprite(SpriteId, SpriteBounds),

    /// Creates a dynamic texture that updates if the sprite updates or if the canvas resolution changes
    /// The texture is created so that it can cover a region specified by CanvasSize in the current coordinate scheme
    /// with a 1-to-1 pixel mapping
    CreateDynamicSprite(SpriteId, SpriteBounds, CanvasSize),

    /// Sets the transparency to use when rendering a texture
    FillTransparency(f32),
}
