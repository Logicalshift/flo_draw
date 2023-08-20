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
    fn to_rgba_u8_slice(&self) -> &[u8];
    fn to_rgba_u8_slice_mut(&mut self) -> &mut [u8];
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

#[cfg(test)]
mod test {
    use super::{U8RgbaPremultipliedPixel, ToRgbaU8Slice};

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
    fn convert_u8_pixels() {
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
    fn write_u8_pixels() {
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
}