use super::alpha_blend_trait::*;
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
    Self: Neg<Output=Self> + Add<Output=Self> + Sub<Output=Self> + Mul<Output=Self> + Div<Output=Self>,
    Self: Add<Self::Component, Output=Self> + Sub<Self::Component, Output=Self> + Mul<Self::Component, Output=Self> + Div<Self::Component, Output=Self>,
    Self: AlphaBlend,
    Self: ToGammaColorSpace<U8RgbaPremultipliedPixel>,
{
    type Component: Sized + Copy + Clone + AlphaValue + Neg<Output=Self::Component> + Add<Output=Self::Component> + Sub<Output=Self::Component> + Mul<Output=Self::Component> + Div<Output=Self::Component>;

    /// A pixel representing the 'black' colour
    fn black() -> Self;

    /// A pixel representing the 'white' colour
    fn white() -> Self;

    /// Creates a pixel from a canvas colour with a particular gamma correction value (2.2 is standard on OS X and windows)
    fn from_color(color: canvas::Color, gamma: f64) -> Self;

    /// Converts this pixel colour back to a canvas colour (2.2 is the standard gamma value on OS X and windows)
    fn to_color(&self, gamma: f64) -> canvas::Color;

    /// Returns the components that make up this pixel
    fn to_components(&self) -> [Self::Component; N];

    /// Retrieves an individual component from this 
    fn get(&self, component: usize) -> Self::Component { self.to_components()[component] }

    /// Returns the alpha component of this pixel
    fn alpha_component(&self) -> Self::Component;
}
