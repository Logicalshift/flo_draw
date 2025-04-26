use std::slice;

///
/// Pixel structure that represents a pixel stored as a U32 in the format `AAAAAAAARRRRRRRRGGGGGGGGBBBBBBBB`
///
/// This is an 8BPP format that is used by some external libraries. Values are store as premultiplied non-linear
///
/// Note that this is different from U32LinearPixel, which stores its pixels as 4 U32 values.
///
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct U32ArgbPremultipliedPixel(u32);

impl U32ArgbPremultipliedPixel {
    ///
    /// Sets a pixel value from a u32 value containing the ARGB values
    ///
    #[inline]
    pub fn from_u32_argb(components: u32) -> Self {
        U32ArgbPremultipliedPixel(components)
    }

    ///
    /// Creates a U32 Argb pixel from RGBA components
    ///
    #[inline]
    pub fn from_rgba_components(r: u8, g: u8, b: u8, a: u8) -> Self {
        let val = ((r as u32) << 16)
                | ((g as u32) << 8)
                | ((b as u32) << 0)
                | ((a as u32) << 24);

        U32ArgbPremultipliedPixel(val)
    }

    ///
    /// Returns the RGBA components in this pixel
    ///
    #[inline]
    pub fn to_rgba_components(self) -> [u8; 4] {
        [
            ((self.0 >> 16) & 0xff) as u8,
            ((self.0 >> 8) & 0xff) as u8,
            ((self.0 >> 0) & 0xff) as u8,
            ((self.0 >> 24) & 0xff) as u8,
        ]
    }
}

impl Default for U32ArgbPremultipliedPixel {
    #[inline]
    fn default() -> Self {
        U32ArgbPremultipliedPixel(0)
    }
}

pub trait ToU32ArgbPremultipliedPixel {
    /// Converts a slice of `u32` values to a slice of U32ArgbPremultipliedPixel
    fn to_argb_slice(&self) -> &[U32ArgbPremultipliedPixel];

    /// Converts a slice of `u32` values to a mutable slice of U32ArgbPremultipliedPixel
    fn to_argb_slice_mut(&mut self) -> &mut [U32ArgbPremultipliedPixel];
}

impl ToU32ArgbPremultipliedPixel for [u32] {
    #[inline]
    fn to_argb_slice(&self) -> &[U32ArgbPremultipliedPixel] {
        unsafe {
            let len     = self.len();
            let data    = self.as_ptr();
            let data    = data as *const U32ArgbPremultipliedPixel;

            slice::from_raw_parts(data, len)
        }
    }

    #[inline]
    fn to_argb_slice_mut(&mut self) -> &mut [U32ArgbPremultipliedPixel] {
        unsafe {
            let len     = self.len();
            let data    = self.as_mut_ptr();
            let data    = data as *mut U32ArgbPremultipliedPixel;

            slice::from_raw_parts_mut(data, len)
        }
    }
}