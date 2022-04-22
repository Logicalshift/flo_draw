use crate::sprite::*;

use std::sync::*;

///
/// Identifier for a texture
///
/// Textures are bitmaps that can be used as fills
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TextureId(pub u64);

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
/// Bitmap filters that can be applied as a post-processing step to textures
///
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum TextureFilter {
    /// Applies a gaussian blur with a given radius
    ///
    /// The radius supplied here is used to calculate the sigma for the blur: a sigma of 0.25 corresponds to a radius of 1.0, 0.5 to a radius of 2.0, etc.
    GaussianBlur(f32),

    ///
    /// Add transparency to the image, where a value of 1.0 is opaque (leave the image as is), and 0.0 is transparent (no image result)
    ///
    AlphaBlend(f32),

    ///
    /// Use the alpha channel of a source texture as a mask for the input texture
    ///
    Mask(TextureId),

    ///
    /// Use the red and green channels of a source texture as a displacement map. The two other parameters are the scale factors (maximum displacement in canvas units)
    ///
    DisplacementMap(TextureId, f32, f32),
}

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

    /// Copies this texture to another texture
    Copy(TextureId),

    /// Applies a filter to this texture. For dynamic textures, this filter will be re-applied any time the texture is rendered.
    /// For dynamic textures, any measurements (eg: gaussian blur radius) are in sprite units, but for static textures, measurements
    /// are in pixels.
    Filter(TextureFilter),
}
