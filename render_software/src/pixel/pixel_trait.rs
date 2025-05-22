use super::alpha_blend_trait::*;
use super::rgba_texture::*;
use super::u16_linear_texture::*;
use super::texture_reader::*;
use super::to_gamma_colorspace_trait::*;
use super::to_linear_colorspace_trait::*;
use super::u8_rgba::*;
use super::u16_rgba::*;

use flo_canvas as canvas;

use std::ops::*;

///
/// Trait implemented by types that represent a pixel. A pixel is a square region of a single colour
///
/// Pixel transforms and operations should be performed in a linear colour space
///
pub trait Pixel<const N: usize>
where
    Self: Sized + Copy + Default + Clone,
    Self: Add<Output=Self> + Sub<Output=Self> + Mul<Output=Self> + Div<Output=Self>,
    Self: Add<Self::Component, Output=Self> + Sub<Self::Component, Output=Self> + Mul<Self::Component, Output=Self> + Div<Self::Component, Output=Self>,
    Self: AlphaBlend,
    Self: ToGammaColorSpace<U8RgbaPremultipliedPixel>,
    Self: ToLinearColorSpace<U16LinearPixel>,
    Self: TextureReader<RgbaTexture>,
    Self: TextureReader<U16LinearTexture>,
{
    /// A pixel representing the 'black' colour
    fn black() -> Self;

    /// A pixel representing the 'white' colour
    fn white() -> Self;

    /// A pixel with the specified components
    fn from_components(components: [Self::Component; N]) -> Self;

    /// Creates a pixel from a canvas colour with a particular gamma correction value (2.2 is standard on OS X and windows)
    fn from_color(color: canvas::Color, gamma: f64) -> Self;

    /// Converts this pixel colour back to a canvas colour (2.2 is the standard gamma value on OS X and windows)
    fn to_color(&self, gamma: f64) -> canvas::Color;

    /// Returns the components that make up this pixel
    fn to_components(&self) -> [Self::Component; N];

    /// Performs bilinear filtering on a set of pixels
    ///
    /// `x` and `y` are in the range `0.0..1.0` and the pixels in the order top-left, top-right, bottom-left, bottom-right (or
    /// `(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)` in x-y coordinates. The return value is the values for the pixels 
    /// blended for the sub-pixel location.
    #[inline]
    fn filter_bilinear(pixels: [&Self; 4], x: Self::Component, y: Self::Component) -> Self {
        let one_minus_x = Self::Component::one()-x;
        let one_minus_y = Self::Component::one()-y;

        let x1 = (*pixels[0])*one_minus_x + (*pixels[1])*x;
        let x2 = (*pixels[2])*one_minus_x + (*pixels[3])*x;

        x1*one_minus_y + x2*y
    }

    /// Retrieves an individual component from this pixel
    fn get(&self, component: usize) -> Self::Component { self.to_components()[component] }
}

///
/// Trait implemented for types that can read from a texture using bilinear interpolation
///
pub trait BilinearTextureReader<TTexture, const N: usize>
where
    Self:       TextureReader<TTexture>,
    TTexture:   Send + Sync
{
    ///
    /// Reads a set of pixels across a linear gradient
    ///
    /// This is a common way that pixels are read out from a texture, so this function can be overridden to optimise this
    /// for different types of texture storage if needed.
    ///
    /// This reads `count` pixels at locations `t = 0, 1, 2, ...` such that `u = dx * t + offset` and `x = x_gradient.0 * u + x_gradient.1`,
    /// `y = y_gradient.0 * u + y_gradient.1`.
    ///
    #[inline]
    fn read_pixels_linear_bilinear_filter(texture: &TTexture, offset:f64, dx: f64, x_gradient: (f64, f64), y_gradient: (f64, f64), count: usize) -> Vec<Self> {
        // Allocate enough space to store the pixels
        let mut positions = Vec::with_capacity(count);

        // Calculate the positions for the pixels
        positions.extend((0..count).map(|t| {
            let t = t as f64;
            let u = dx * t + offset;
            let x = x_gradient.0 * u + x_gradient.1;
            let y = y_gradient.0 * u + y_gradient.1;

            (x, y)
        }));

        Self::read_pixels_bilinear_filter(texture, &positions)
    }

    ///
    /// Reads pixels and applies bilinear filtering to approximate values found at subpixels
    ///
    /// This can be used for scaling up an image or scaling down an image to about half size
    ///
    fn read_pixels_bilinear_filter(texture: &TTexture, positions: &[(f64, f64)]) -> Vec<Self>;
}

impl<TPixel, TTexture, const N: usize> BilinearTextureReader<TTexture, N> for TPixel
where
    TPixel:     Pixel<N> + TextureReader<TTexture>,
    TTexture:   Send + Sync
{
    ///
    /// Reads pixels and applies bilinear filtering to approximate values found at subpixels
    ///
    /// This can be used for scaling up an image or scaling down an image to about half size
    ///
    fn read_pixels_bilinear_filter(texture: &TTexture, positions: &[(f64, f64)]) -> Vec<Self> {
        // Create the resulting pixels for this read
        let mut result          = Vec::with_capacity(positions.len());

        // If there's nothing to read, then short-circuit
        if positions.is_empty() {
            return result;
        }

        // In order to minimize the amount of reading we do, we make a plan of the pixels we're going to read (for each pixel in the output we need a 2x2 sample set)
        enum Action {
            NextQuad,
            ReadPixel(f64, f64),
        }
        let mut actions         = Vec::with_capacity(positions.len()*2);
        let mut pixels_to_read  = Vec::with_capacity(positions.len()*4);

        // We always start by reading the first position (the 4 pixels surrounding xpos, ypos)
        let (mut xpos, mut ypos) = positions[0];

        xpos = xpos.floor();
        ypos = ypos.floor();

        pixels_to_read.extend([
            (xpos, ypos), (xpos+1.0, ypos), (xpos, ypos+1.0), (xpos+1.0, ypos+1.0)
        ]);

        // Generate the actions and the pixels to read
        for (next_x, next_y) in positions {
            // Read another set of pixels if the current pixel doesn't match
            if xpos != next_x.floor() || ypos != next_y.floor() {
                xpos = next_x.floor();
                ypos = next_y.floor();

                pixels_to_read.extend([
                    (xpos, ypos), (xpos+1.0, ypos), (xpos, ypos+1.0), (xpos+1.0, ypos+1.0)
                ]);
                actions.push(Action::NextQuad);
            }

            // Interpolate the next pixel
            actions.push(Action::ReadPixel(next_x - xpos, next_y - ypos));
        }

        // Read the data we need from the texture and then perform the actions
        let source_pixels       = Self::read_pixels(texture, &pixels_to_read);
        let mut pos             = 0;
        let mut current_pixels  = [&source_pixels[0], &source_pixels[1], &source_pixels[2], &source_pixels[3]];

        for action in actions {
            match action {
                Action::NextQuad => {
                    pos             += 4;
                    current_pixels  = [&source_pixels[pos+0], &source_pixels[pos+1], &source_pixels[pos+2], &source_pixels[pos+3]];
                },

                Action::ReadPixel(offset_x, offset_y) => {
                    result.push(Self::filter_bilinear(current_pixels, TPixel::Component::with_value(offset_x), TPixel::Component::with_value(offset_y)));
                }
            }
        }

        result
    }
}
