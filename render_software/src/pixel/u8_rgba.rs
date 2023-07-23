///
/// An RGBA pixel as a set of u8 values
///
/// The alpha value is pre-multiplied into the RGB values, and the colour space is gamma-corrected
///
pub struct U8RgbaPremultipliedPixel([u8; 4]);

impl U8RgbaPremultipliedPixel {
    ///
    /// Creates a new U8 pixel from the component bytes
    ///
    #[inline]
    pub fn from_components(components: [u8; 4]) -> Self {
        U8RgbaPremultipliedPixel(components)
    }

    ///
    /// Retrieves the RGBA values from this pixel
    ///
    #[inline]
    pub fn get_components(&self) -> [u8; 4] {
        self.0
    }
}
