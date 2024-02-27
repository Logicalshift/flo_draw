use std::slice;

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

    ///
    /// Reinterprets a slice of u16 values as a set of U16LinearPixel values
    ///
    #[inline]
    pub fn u16_slice_as_linear_pixels(data: &mut [u16]) -> &mut [U16LinearPixel] {
        unsafe {
            let len     = data.len();
            let data    = data.as_mut_ptr();
            let data    = data as *mut U16LinearPixel;

            slice::from_raw_parts_mut(data, len/4)
        }
    }
}

impl Default for U16LinearPixel {
    #[inline]
    fn default() -> Self {
        U16LinearPixel([0, 0, 0, 0])
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn write_u16_buffer_using_pixels() {
        let mut u16values   = [0u16; 256];
        let pixel_values    = U16LinearPixel::u16_slice_as_linear_pixels(&mut u16values);

        for pos in 0..(256/4) {
            let x = pos as u16;
            pixel_values[pos] = U16LinearPixel::from_components([x, x+1, x+2, x+3]);
        }

        for pos in 0..(256/4) {
            let pixel = &u16values[(pos*4)..(pos*4+4)];
            let x = pos as u16;

            assert!(pixel == [x, x+1, x+2, x+3]);
        }
    }
}