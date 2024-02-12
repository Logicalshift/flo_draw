use std::convert::{TryFrom};

///
/// An RGBA texture where the values are stored as linear intensities between 0-65535, with the alpha value
/// multiplied in.
///
/// This can be rendered much more quickly than the 8-bit RGBA texture, which
///
pub struct U16LinearTexture {
    /// The width of the texture in pixels (a row is 4x this value)
    width: i64,

    /// The height of the texture in pixels
    height: i64,

    /// The pixels stored for this texture, as a 
    pixels: Vec<u16>,
}

impl U16LinearTexture {
    ///
    /// Creates a U16 texture by loading pixel RGBA values
    ///
    /// There should width * height * 4 pixels. (A pixel address is found by `addr = y * (width + x)*4`)
    ///
    #[inline]
    pub fn from_pixels(width: usize, height: usize, pixels: Vec<u16>) -> Self{
        // Width and height are stored as i64s as the texture needs to be able to wrap around
        let width   = width as i64;
        let height  = height as i64;

        Self { width, height, pixels }
    }

    ///
    /// The width of this texture
    ///
    pub fn width(&self) -> usize {
        self.width as usize
    }

    ///
    /// The height of this texture
    ///
    pub fn height(&self) -> usize {
        self.height as usize
    }

    ///
    /// Returns the pixels in this texture
    ///
    #[inline]
    pub fn pixels(&self) -> &[u16] {
        &self.pixels
    }

    ///
    /// Calculates the index of a pixel in the pixels array for a given x and y position
    ///
    /// The x, y positions wrap around
    ///
    #[inline]
    pub fn pixel_index(&self, x: i64, y: i64) -> usize {
        // The texture is treated as repeating infinitely
        let x   = if x >= 0 { x%self.width } else { (x%self.width) + self.width };
        let y   = if y >= 0 { y%self.height } else { (y%self.height) + self.height };

        // Calculate the index where this pixel is
        let idx = (x + y*self.width) * 4;
        let idx = idx as usize;

        // Must fit within the size of the pixels list
        debug_assert!(idx + 4 <= self.pixels.len());

        idx
    }

    ///
    /// Reads a pixel from the texture
    ///
    /// The result is a premultiplied set of R, G, B, A values
    ///
    #[inline]
    pub fn read_pixel(&self, x: i64, y: i64) -> &[u16; 4] {
        let idx     = self.pixel_index(x, y);
        let pixels  = &self.pixels[idx..(idx+4)];

        <&[u16; 4]>::try_from(pixels).unwrap()
    }
}
