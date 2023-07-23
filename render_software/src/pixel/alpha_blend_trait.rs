use std::ops::*;

///
/// The alpha blending functions that can be applied to a 
///
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlphaFunction {
    Zero,
    One,
    SourceAlpha,
    DestAlpha,
    OneMinusSourceAlpha,
    OneMinusDestAlpha,
}

///
/// An operation applied to an alpha function
///
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlphaOperation {
    Clear,
    Source,
    Target,
    SourceOver,
    DestOver,
    SourceIn,
    DestIn,
    SourceHeldOut,
    DestHeldOut,
    SourceAtop,
    DestAtop,
    Xor,
}

///
/// Trait implemented by types that support alpha blending
///
pub trait AlphaBlend : Sized {
    /// Performs alpha blending with a chosen source and target functions (for premultiplied alphas)
    fn alpha_blend_with_function(self, dest: Self, source_alpha: AlphaFunction, dest_alpha: AlphaFunction) -> Self;

    /// Performs the specified alpha blending operation
    #[inline]
    fn alpha_blend(self, dest: Self, operation: AlphaOperation) -> Self {
        let (src, dst) = operation.functions();
        self.alpha_blend_with_function(dest, src, dst)
    }

    #[inline] fn source_over(self, dest: Self) -> Self        { self.alpha_blend(dest, AlphaOperation::SourceOver) }
    #[inline] fn dest_over(self, dest: Self) -> Self          { self.alpha_blend(dest, AlphaOperation::DestOver) }
    #[inline] fn source_in(self, dest: Self) -> Self          { self.alpha_blend(dest, AlphaOperation::SourceIn) }
    #[inline] fn dest_in(self, dest: Self) -> Self            { self.alpha_blend(dest, AlphaOperation::DestIn) }
    #[inline] fn source_held_out(self, dest: Self) -> Self    { self.alpha_blend(dest, AlphaOperation::SourceHeldOut) }
    #[inline] fn dest_held_out(self, dest: Self) -> Self      { self.alpha_blend(dest, AlphaOperation::DestHeldOut) }
    #[inline] fn source_atop(self, dest: Self) -> Self        { self.alpha_blend(dest, AlphaOperation::SourceAtop) }
    #[inline] fn dest_atop(self, dest: Self) -> Self          { self.alpha_blend(dest, AlphaOperation::DestAtop) }
}

impl AlphaOperation {
    ///
    /// Returns the alpha functions to use for the source and target for this alpha operation
    ///
    #[inline]
    pub const fn functions(&self) -> (AlphaFunction, AlphaFunction) {
        match self {
            AlphaOperation::Clear           => (AlphaFunction::Zero,                AlphaFunction::Zero),
            AlphaOperation::Source          => (AlphaFunction::One,                 AlphaFunction::Zero),
            AlphaOperation::Target          => (AlphaFunction::Zero,                AlphaFunction::One),
            AlphaOperation::SourceOver      => (AlphaFunction::One,                 AlphaFunction::OneMinusSourceAlpha),
            AlphaOperation::DestOver        => (AlphaFunction::OneMinusDestAlpha,   AlphaFunction::One),
            AlphaOperation::SourceIn        => (AlphaFunction::DestAlpha,           AlphaFunction::Zero),
            AlphaOperation::DestIn          => (AlphaFunction::Zero,                AlphaFunction::SourceAlpha),
            AlphaOperation::SourceHeldOut   => (AlphaFunction::OneMinusDestAlpha,   AlphaFunction::Zero),
            AlphaOperation::DestHeldOut     => (AlphaFunction::Zero,                AlphaFunction::OneMinusSourceAlpha),
            AlphaOperation::SourceAtop      => (AlphaFunction::DestAlpha,           AlphaFunction::OneMinusSourceAlpha),
            AlphaOperation::DestAtop        => (AlphaFunction::OneMinusDestAlpha,   AlphaFunction::SourceAlpha),
            AlphaOperation::Xor             => (AlphaFunction::OneMinusDestAlpha,   AlphaFunction::OneMinusSourceAlpha),
        }
    }
}

///
/// Returns the 0 and 1 values for an alpha component
///
pub trait AlphaValue {
    fn zero() -> Self;
    fn one() -> Self;
}

impl AlphaFunction {
    ///
    /// Applies this alpha function to a pixel
    ///
    #[inline]
    pub fn apply<TPixel, TComponent>(&self, pixel: TPixel, src_alpha: TComponent, dst_alpha: TComponent) -> TPixel
    where
        TPixel:         Copy + Mul<TComponent, Output=TPixel>,
        TComponent:     Copy + AlphaValue + Sub<TComponent, Output=TComponent>,
    {
        match self {
            AlphaFunction::Zero                     => pixel * TComponent::zero(),
            AlphaFunction::One                      => pixel * TComponent::one(),
            AlphaFunction::SourceAlpha              => pixel * src_alpha,
            AlphaFunction::DestAlpha                => pixel * dst_alpha,
            AlphaFunction::OneMinusSourceAlpha      => pixel * (TComponent::one() - src_alpha),
            AlphaFunction::OneMinusDestAlpha        => pixel * (TComponent::one() - dst_alpha),
        }
    }
}

impl AlphaValue for f32 {
    #[inline] fn zero() -> f32 { 0.0 }
    #[inline] fn one() -> f32 { 1.0 }
}

impl AlphaValue for f64 {
    #[inline] fn zero() -> f64 { 0.0 }
    #[inline] fn one() -> f64 { 1.0 }
}
