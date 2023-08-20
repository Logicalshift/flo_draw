use std::slice;

///
/// An RGBA pixel as a set of u8 values
///
/// The alpha value is pre-multiplied into the RGB values, and the colour space is gamma-corrected
///
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(C)]
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

impl Default for U8RgbaPremultipliedPixel {
    #[inline]
    fn default() -> Self {
        U8RgbaPremultipliedPixel([0, 0, 0, 0])
    }
}

pub trait ToRgbaU8Slice {
    /// Returns the pixels as a single slice of u8 values
    fn to_rgba_u8_slice(&self) -> &[u8];

    /// Returns the pixels as a mutable single slice of u8 values
    fn to_rgba_u8_slice_mut(&mut self) -> &mut [u8];
}

pub trait ToRgbaPremultipliedPixels {
    /// Converts a slice of `u8` values to a slice of U8RgbaPremultipliedPixels
    ///
    /// If the slice is not a multiple of 4, then no pixels are generated at the end
    fn to_rgba_slice(&self) -> &[U8RgbaPremultipliedPixel];

    /// Converts a slice of `u8` values to a mutable slice of U8RgbaPremultipliedPixels
    fn to_rgba_slice_mut(&mut self) -> &mut [U8RgbaPremultipliedPixel];
}

impl ToRgbaU8Slice for [U8RgbaPremultipliedPixel] {
    #[inline]
    fn to_rgba_u8_slice(&self) -> &[u8] {
        unsafe {
            let len     = self.len();
            let data    = self.as_ptr();
            let data    = data as *const u8;

            slice::from_raw_parts(data, len*4)
        }
    }

    #[inline]
    fn to_rgba_u8_slice_mut(&mut self) -> &mut [u8] {
        unsafe {
            let len     = self.len();
            let data    = self.as_mut_ptr();
            let data    = data as *mut u8;

            slice::from_raw_parts_mut(data, len*4)
        }
    }
}

impl ToRgbaPremultipliedPixels for [u8] {
    #[inline]
    fn to_rgba_slice(&self) -> &[U8RgbaPremultipliedPixel] {
        unsafe {
            let len     = self.len();
            let data    = self.as_ptr();
            let data    = data as *const U8RgbaPremultipliedPixel;

            slice::from_raw_parts(data, len/4)
        }
    }

    #[inline]
    fn to_rgba_slice_mut(&mut self) -> &mut [U8RgbaPremultipliedPixel] {
        unsafe {
            let len     = self.len();
            let data    = self.as_mut_ptr();
            let data    = data as *mut U8RgbaPremultipliedPixel;

            slice::from_raw_parts_mut(data, len/4)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn rgba(t: f64) -> [u8; 4] {
        let r = t * 0.3;
        let g = t * 0.3 + 0.3;
        let b = t * 0.3 + 0.6;
        let a = t * 0.1 + 0.9;

        let r = (r*255.0).floor() as u8;
        let g = (g*255.0).floor() as u8;
        let b = (b*255.0).floor() as u8;
        let a = (a*255.0).floor() as u8;

        [r, g, b, a]
    }

    #[test]
    fn read_rgba_as_u8() {
        // Fill up some pixels
        let mut pixel_data = vec![];
        for x in 0..65536 {
            let t = (x as f64)/65536.0;

            pixel_data.push(U8RgbaPremultipliedPixel::from_components(rgba(t)));
        }

        // Try out the conversion routine
        let as_u8_array = pixel_data.to_rgba_u8_slice();
        for x in 0..65536 {
            let pos = x * 4;
            let t = (x as f64)/65536.0;
            let [r, g, b, a] = rgba(t);

            assert!(as_u8_array[pos+0] == r);
            assert!(as_u8_array[pos+1] == g);
            assert!(as_u8_array[pos+2] == b);
            assert!(as_u8_array[pos+3] == a);
        }
    }

    #[test]
    fn write_rgba_using_u8() {
        // Fill up some pixels
        let mut pixel_data = vec![];
        for _ in 0..65536 {
            pixel_data.push(U8RgbaPremultipliedPixel::default());
        }

        // Fill using the mutable version of the function
        let as_u8_array = pixel_data.to_rgba_u8_slice_mut();
        for x in 0..65536 {
            let pos = x * 4;
            let t = (x as f64)/65536.0;
            let [r, g, b, a] = rgba(t);

            as_u8_array[pos+0] = r;
            as_u8_array[pos+1] = g;
            as_u8_array[pos+2] = b;
            as_u8_array[pos+3] = a;
        }

        // Check that the original vec looks OK
        for x in 0..65536 {
            let t = (x as f64)/65536.0;

            assert!(pixel_data[x] == U8RgbaPremultipliedPixel::from_components(rgba(t)));
        }
    }

    #[test]
    fn read_u8_buffer_using_rgba() {
        // Fill up some pixels
        let mut pixel_data  = vec![0u8; 65536*4];
        for x in 0..65536 {
            let pos = x * 4;
            let t = (x as f64)/65536.0;
            let [r, g, b, a] = rgba(t);

            pixel_data[pos+0] = r;
            pixel_data[pos+1] = g;
            pixel_data[pos+2] = b;
            pixel_data[pos+3] = a;
        }

        // Should read back as expected the u8 array
        let as_rgba_array   = pixel_data.to_rgba_slice();
        for x in 0..65536 {
            let t = (x as f64)/65536.0;

            assert!(as_rgba_array[x] == U8RgbaPremultipliedPixel::from_components(rgba(t)));
        }
    }

    #[test]
    fn write_u8_buffer_using_rgba() {
        // Fill up some pixels
        let mut pixel_data  = vec![0u8; 65536*4];
        let as_rgba_array   = pixel_data.to_rgba_slice_mut();
        for x in 0..65536 {
            let t = (x as f64)/65536.0;

            as_rgba_array[x] = U8RgbaPremultipliedPixel::from_components(rgba(t));
        }

        // Should read back as expected the u8 array
        for x in 0..65536 {
            let pos = x * 4;
            let t = (x as f64)/65536.0;
            let [r, g, b, a] = rgba(t);

            assert!(pixel_data[pos+0] == r);
            assert!(pixel_data[pos+1] == g);
            assert!(pixel_data[pos+2] == b);
            assert!(pixel_data[pos+3] == a);
        }
    }
}
