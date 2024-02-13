use super::alpha_blend_trait::*;
use super::rgba_texture::*;
use super::texture_reader::*;
use super::to_gamma_colorspace_trait::*;
use super::u8_rgba::*;

use flo_canvas as canvas;

use std::ops::*;

///
/// Trait implemented by types that represent a pixel. A pixel is a square region of a single colour
///
/// Pixel transforms and operations should be performed in a linear colour space
///
pub trait Pixel<const N: usize>
where
    Self: Sized + Copy + Clone,
    Self: Add<Output=Self> + Sub<Output=Self> + Mul<Output=Self> + Div<Output=Self>,
    Self: Add<Self::Component, Output=Self> + Sub<Self::Component, Output=Self> + Mul<Self::Component, Output=Self> + Div<Self::Component, Output=Self>,
    Self: AlphaBlend,
    Self: ToGammaColorSpace<U8RgbaPremultipliedPixel>,
    Self: TextureReader<RgbaTexture>,
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

        x1*y + x2*one_minus_y
    }

    /// Retrieves an individual component from this pixel
    fn get(&self, component: usize) -> Self::Component { self.to_components()[component] }
}
