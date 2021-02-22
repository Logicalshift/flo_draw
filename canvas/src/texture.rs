use std::sync::*;

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
    Create(u32, u32, TextureFormat),

    /// Sets a region of a texture (specified as minx, miny, width, height) to the specified bitmap
    SetBytes(u32, u32, u32, u32, Arc<Vec<u8>>)
}
