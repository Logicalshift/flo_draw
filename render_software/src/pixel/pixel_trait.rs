use super::alpha_blend_trait::*;

use std::ops::*;

///
/// Trait implemented by types that represent a pixel
///
pub trait Pixel<const N: usize>
where
    Self: Sized + Copy + Clone,
    Self: Neg<Output=Self> + Add<Output=Self> + Sub<Output=Self> + Mul<Output=Self> + Div<Output=Self>,
    Self: AlphaBlend
{
    type Component: Sized + Copy + Clone + Neg<Output=Self::Component> + Add<Output=Self::Component> + Sub<Output=Self::Component> + Mul<Output=Self::Component> + Div<Output=Self::Component>;

    /// Returns the components that make up this pixel
    fn to_components(&self) -> [Self::Component; N];

    /// Retrieves an individual component from this 
    fn get(&self, component: usize) -> Self::Component { self.to_components()[component] }
}
