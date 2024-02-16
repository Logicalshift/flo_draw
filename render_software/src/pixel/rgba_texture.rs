use super::u16_linear_texture::*;

use std::convert::{TryFrom};

///
/// An 8-bpp, non-premultiplied RGBA texture
///
/// We also assume a gamma correction value of 2.2 for this texture type
///
#[derive(Clone)]
pub struct RgbaTexture {
    /// The width of the texture in pixels (a row is 4x this value)
    width: i64,

    /// The height of the texture in pixels
    height: i64,

    /// The pixels stored for this texture
    pixels: Vec<u8>,
}

impl RgbaTexture {
    ///
    /// Creates a new RGBA texture
    ///
    pub fn from_pixels(width: usize, height: usize, pixels: Vec<u8>) -> RgbaTexture {
        // SAFETY: we later rely on this to use get_unchecked
        assert!(width * height * 4 == pixels.len());

        RgbaTexture { 
            width:  width as i64, 
            height: height as i64, 
            pixels: pixels
        }
    }

    ///
    /// Creates an RGBA texture from a premultiplied linear texture
    ///
    pub fn from_linear_texture(texture: &U16LinearTexture, gamma: f64) -> RgbaTexture {
        let mut pixels = Vec::with_capacity(texture.width()*texture.height() * 4);

        // Create a look-up table for converting u16 values to gamma-corrected u8 values
        let inverse_gamma   = 1.0 / gamma;
        let gamma_lut       = (0u16..=65535u16).map(|t| (((t as f64 / 65535.0).powf(inverse_gamma) * 255.0).round()) as u8).collect::<Box<[u8]>>();

        for ypos in 0..texture.height() {
            // Read the pixels on this line
            let pixel_line = texture.read_pixels((0..texture.width()).map(|xpos| (xpos as _, ypos as _)));

            // Convert to 8bpp
            for [r, g, b, a] in pixel_line {
                // Convert to u32 to do some fixed-point conversions
                let r = *r as u32;
                let g = *g as u32;
                let b = *b as u32;
                let a = *a as u32;

                // Remove premultiplication from the rgba values
                let inverse_a = if a != 0 { ((65535<<16) / a) >> 16 } else { 0 };
                let r = r * inverse_a;
                let g = g * inverse_a;
                let b = b * inverse_a;

                // Perform a lookup to get the 8-bit values
                let r = gamma_lut[r as usize];
                let g = gamma_lut[g as usize];
                let b = gamma_lut[b as usize];
                let a = (a >> 8) as u8;

                // Add to the pixels
                pixels.extend([r, g, b, a]);
            }
        }

        RgbaTexture {
            width:  texture.width() as _,
            height: texture.height() as _,
            pixels: pixels,
        }
    }

    ///
    /// The width of this texture in pixels
    ///
    #[inline]
    pub fn width(&self) -> usize {
        self.width as usize
    }

    ///
    /// The height of this texture in pixels
    ///
    #[inline]
    pub fn height(&self) -> usize {
        self.height as usize
    }

    ///
    /// The pixels for this texture
    ///
    #[inline]
    pub fn pixels(&self) -> &Vec<u8> {
        &self.pixels
    }

    ///
    /// The pixels for this texture
    ///
    #[inline]
    pub fn pixels_mut(&mut self) -> &mut Vec<u8> {
        &mut self.pixels
    }

    ///
    /// Sets the bytes for a region of this image
    ///
    #[inline]
    pub fn set_bytes(&mut self, x: usize, y: usize, width: usize, height: usize, bytes: &Vec<u8>) {
        // Number of bytes to read/write at a time
        let mut read_bytes = width * 4;

        // Number of bytes to skip after every read_bytes
        let mut read_skip_bytes = 0;

        // Clip to the size of the image
        let x       = x.min(self.width as _);
        let y       = y.min(self.height as _);
        let width   = if x + width > self.width as _ {
            let clip        = (x + width) - self.width as usize;
            read_bytes      = (width - clip) * 4;
            read_skip_bytes = clip * 4;
            width - clip
        } else {
            width 
        };
        let height  = height.min(self.height as usize - y);

        // After writing read_bytes, skip this many bytes to write
        let write_skip_bytes = (self.width as usize - width) * 4;

        // Pointers for reading/writing
        let mut write_idx   = (x*4) + (y*(self.width as usize)*4);
        let mut read_idx    = 0;

        for _ in 0..height {
            // Write a row
            for _ in 0..read_bytes {
                self.pixels[write_idx] = bytes[read_idx];
                write_idx   += 1;
                read_idx    += 1;
            }

            // Skip to the next row
            read_idx    += read_skip_bytes;
            write_idx   += write_skip_bytes;
        }
    }

    ///
    /// Finds a pixel at the specified coordinate in this texture
    ///
    #[inline]
    pub fn read_pixel(&self, x: i64, y: i64) -> &[u8; 4] {
        // The texture is treated as repeating infinitely
        let x   = if x >= 0 { x%self.width } else { (x%self.width) + self.width };
        let y   = if y >= 0 { y%self.height } else { (y%self.height) + self.height };

        // Calculate the index where this pixel is
        let idx     = (x + y*self.width) * 4;
        let idx     = idx as usize;
        let pixels  = &self.pixels;

        // Because of the assertion in new() we know that 'idx' must be in the range covered by this texture
        debug_assert!(idx + 4 <= pixels.len());
        <&[u8; 4]>::try_from(&pixels[idx..idx+4]).unwrap()
    }

    ///
    /// Reads a set of pixels at arbitrary coordinates from the texture
    ///
    #[inline]
    pub fn read_pixels(&self, coords: impl Iterator<Item=(i64, i64)>) -> impl Iterator<Item=&[u8; 4]> {
        coords.map(move |(x, y)| self.read_pixel(x, y))
    }
}
