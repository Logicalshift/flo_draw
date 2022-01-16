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

    /// Sets the transparency to use when rendering a texture
    FillTransparency(f32),
}
