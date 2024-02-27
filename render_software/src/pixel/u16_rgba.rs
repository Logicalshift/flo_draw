///
/// An RGBA pixel as a set of u16 values
///
/// The alpha value is pre-multiplied into the RGB values, and the colour space is linear
///
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct U16LinearPixel([u16; 4]);

impl U16LinearPixel {
    ///
    /// Creates a new U16 pixel from the component bytes
    ///
    #[inline]
    pub fn from_components(components: [u16; 4]) -> Self {
        U16LinearPixel(components)
    }

    ///
    /// Retrieves the RGBA values from this pixel
    ///
    #[inline]
    pub fn get_components(&self) -> [u16; 4] {
        self.0
    }
}

impl Default for U16LinearPixel {
    #[inline]
    fn default() -> Self {
        U16LinearPixel([0, 0, 0, 0])
    }
}
