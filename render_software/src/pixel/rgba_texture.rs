///
/// An 8-bpp, non-premultiplied RGBA texture
///
/// We also assume a gamma correction value of 2.2 for this texture type
///
#[derive(Clone)]
pub struct RgbaTexture {
    /// The width of the texture in pixels (a row is 4x this value)
    pub width: usize,

    /// The height of the texture in pixels
    pub height: usize,

    /// The pixels stored for this texture
    pub pixels: Vec<u8>,
}
