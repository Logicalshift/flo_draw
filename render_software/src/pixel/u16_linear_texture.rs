use super::pixel_trait::*;
use super::rgba_texture::*;
use super::u32_linear::*;
use super::u32_fixed_point::*;

use std::cell::{RefCell};
use std::convert::{TryFrom};

///
/// An RGBA texture where the values are stored as linear intensities between 0-65535, with a premultipled alpha.
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
    pub fn from_pixels(width: usize, height: usize, pixels: Vec<u16>) -> Self {
        // Width and height are stored as i64s as the texture needs to be able to wrap around
        let width   = width as i64;
        let height  = height as i64;

        Self { width, height, pixels }
    }

    ///
    /// Converts from an 8bpp texture with the given gamma value to a linear texture
    ///
    pub fn from_rgba(texture: &RgbaTexture, gamma: f64) -> Self {
        thread_local! {
            /// The look-up table used to map from pixel values + alpha to premultiplied linear values (cached in the current thread in case we're loading a lot textures)
            pub static GAMMA_LUT: RefCell<[u16; 65536]> = RefCell::new([0; 65536]);

            /// The gamma value that's loaded into the table
            pub static CURRENT_GAMMA: RefCell<f64> = RefCell::new(f64::MIN);
        };

        // Borrow the thread local tables
        GAMMA_LUT.with(|gamma_lut| CURRENT_GAMMA.with(|current_gamma| {
            let mut gamma_lut       = gamma_lut.borrow_mut();
            let mut current_gamma   = current_gamma.borrow_mut();

            // Regenerate the look up table if the gamma value doesn't match
            if *current_gamma != gamma {
                *current_gamma = gamma;

                for a in 0..256 {
                    // Convert the alpha value to f64 (these are always linear)
                    let alpha = (a as f64)/255.0;

                    for c in 0..256 {
                        // Gamma correct the value and pre-multiply it
                        let val = (c as f64)/255.0;
                        let val = val.powf(gamma);
                        let val = val * alpha;
                        let val = (val * 65535.0) as u16;

                        // Store in the table
                        let table_pos = (a<<8) | c;
                        gamma_lut[table_pos] = val;
                    }
                }
            }

            // Create the result by applying gamma correction to the Rgba texture
            let width       = texture.width();
            let height      = texture.height();
            let mut pixels  = Vec::with_capacity(width * height * 4);

            for y in 0..height {
                for [r, g, b, a] in texture.read_pixels((0..width).map(|x| (x as i64, y as i64))) {
                    let alpha   = (*a as usize) << 8;
                    let ra      = (*r as usize) | alpha;
                    let ga      = (*g as usize) | alpha;
                    let ba      = (*b as usize) | alpha;

                    pixels.extend([
                        gamma_lut[ra],
                        gamma_lut[ga],
                        gamma_lut[ba],
                        (*a as u16) * 257
                    ])
                }
            }

            // Result is a new linear texture
            U16LinearTexture { 
                width:  width as i64, 
                height: height as i64, 
                pixels: pixels,
            }
        }))
    }

    ///
    /// Creates a lower mip-map level from this texture
    ///
    pub fn create_mipmap(&self) -> Option<Self> {
        if self.width == 0 || self.height == 0 {
            // Texture is already empty
            None
        } else if self.width == 1 && self.height == 1 {
            // We can't half the size of this texture
            None
        } else {
            // Generate a texture of half the size of the original (treating it as if it wraps around)
            let width       = self.width as usize;
            let height      = self.height as usize;

            let x_wrap      = width % 2;
            let y_wrap      = height % 2;
            let new_width   = width / 2 + x_wrap;
            let new_height  = height / 2 + y_wrap;

            let mut new_pixels = Vec::with_capacity((new_width * new_height * 4) as usize);

            let mut upper_pixels = vec![U32LinearPixel::black(); width+x_wrap];
            let mut lower_pixels = vec![U32LinearPixel::black(); width+x_wrap];

            // Blend the pixels from this texture to create a half-size texture
            for y_pos in 0..new_height {
                // Fetch the two rows to combine
                let offset      = y_pos * (width * 4 * 2);
                let offset_2    = offset + (width * 4);
                let offset_2    = if offset_2 >= self.pixels.len() { 0 } else { offset_2 };
                let upper_row   = &self.pixels[offset..(offset+(width*4))];
                let lower_row   = &self.pixels[offset_2..(offset_2+(width*4))];

                // Convert to U32LinearPixels
                for x_pos in 0..width {
                    // Read the rgba values for the upper and lower rows
                    let r1 = upper_row[(x_pos*4)+0];
                    let g1 = upper_row[(x_pos*4)+1];
                    let b1 = upper_row[(x_pos*4)+2];
                    let a1 = upper_row[(x_pos*4)+3];

                    let r2 = lower_row[(x_pos*4)+0];
                    let g2 = lower_row[(x_pos*4)+1];
                    let b2 = lower_row[(x_pos*4)+2];
                    let a2 = lower_row[(x_pos*4)+3];

                    // Convert and store the pixels
                    upper_pixels[x_pos] = U32LinearPixel::from_components([r1.into(), g1.into(), b1.into(), a1.into()]);
                    lower_pixels[x_pos] = U32LinearPixel::from_components([r2.into(), g2.into(), b2.into(), a2.into()]);
                }

                // Add the x wrap-around pixel if there is one
                if x_wrap == 1 {
                    let r1 = upper_row[0];
                    let g1 = upper_row[1];
                    let b1 = upper_row[2];
                    let a1 = upper_row[3];

                    let r2 = lower_row[0];
                    let g2 = lower_row[1];
                    let b2 = lower_row[2];
                    let a2 = lower_row[3];

                    upper_pixels[width] = U32LinearPixel::from_components([r1.into(), g1.into(), b1.into(), a1.into()]);
                    lower_pixels[width] = U32LinearPixel::from_components([r2.into(), g2.into(), b2.into(), a2.into()]);
                }

                // Combine the pixels in groups of 4 to generate the result
                let one_quarter = U32FixedPoint(16384);

                for x_pos in 0..new_width {
                    // Average the pixels to create the output pixel
                    let p1 = upper_pixels[x_pos * 2];
                    let p2 = upper_pixels[x_pos * 2+1];
                    let p3 = lower_pixels[x_pos * 2];
                    let p4 = lower_pixels[x_pos * 2+1];

                    let output_pixel = (p1 + p2 + p3 + p4) * one_quarter;
                    let [r, g, b, a] = output_pixel.to_components();

                    new_pixels.push(r.into());
                    new_pixels.push(g.into());
                    new_pixels.push(b.into());
                    new_pixels.push(a.into());
                }
            }

            Some(U16LinearTexture {
                width:  new_width as _,
                height: new_height as _,
                pixels: new_pixels,
            })
        }
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

    ///
    /// Reads a set of pixels at arbitrary coordinates from the texture
    ///
    #[inline]
    pub fn read_pixels(&self, coords: impl Iterator<Item=(i64, i64)>) -> impl Iterator<Item=&[u16; 4]> {
        coords.map(move |(x, y)| self.read_pixel(x, y))
    }
}


impl From<RgbaTexture> for U16LinearTexture {
    #[inline]
    fn from(texture: RgbaTexture) -> U16LinearTexture {
        // Convert using the standard gamma ratio of 2.2
        U16LinearTexture::from_rgba(&texture, 2.2)
    }
}
