use super::alpha_blend_trait::*;
use super::to_gamma_colorspace_trait::*;
use super::pixel_trait::*;
use super::u8_rgba::*;

use flo_canvas as canvas;

use wide::*;
use once_cell::sync::{Lazy};

use std::ops::*;

///
/// A pixel using linear floating-point components, with the alpha value pre-multiplied
///
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct F32LinearPixel(f32x4);

impl Default for F32LinearPixel {
    #[inline]
    fn default() -> Self {
        F32LinearPixel(f32x4::splat(0.0))
    }
}

impl Pixel<4> for F32LinearPixel {
    type Component = f32;

    #[inline]
    fn black() -> F32LinearPixel {
        F32LinearPixel(f32x4::new([0.0, 0.0, 0.0, 1.0]))
    }

    #[inline]
    fn white() -> F32LinearPixel {
        F32LinearPixel(f32x4::new([1.0, 1.0, 1.0, 1.0]))
    }

    #[inline]
    fn from_components(components: [Self::Component; 4]) -> Self {
        F32LinearPixel(f32x4::new(components))
    }

    #[inline]
    fn to_components(&self) -> [Self::Component; 4] {
        self.0.to_array()
    }

    #[inline]
    fn get(&self, component: usize) -> Self::Component { 
        self.to_components()[component] 
    }

    #[inline]
    fn alpha_component(&self) -> Self::Component {
        self.0.as_array_ref()[3]
    }

    #[inline]
    fn from_color(color: canvas::Color, gamma: f64) -> Self {
        let (r, g, b, a) = color.to_rgba_components();

        // Add premultiplication and gamma correction
        let gamma = gamma as f32;
        let pixel = f32x4::new([r, g, b, a]);
        let pixel = pixel.pow_f32x4(f32x4::new([gamma, gamma, gamma, 1.0]));
        let pixel = pixel * f32x4::new([a, a, a, 1.0]);

        F32LinearPixel(pixel)
    }

    #[inline]
    fn to_color(&self, gamma: f64) -> canvas::Color {
        let alpha   = self.0.as_array_ref()[3];

        // Remove premultiplication and gamma correction
        let gamma   = (1.0/gamma) as f32;
        let rgba    = self.0 / f32x4::new([alpha, alpha, alpha, 1.0]);
        let rgba    = rgba.pow_f32x4(f32x4::new([gamma, gamma, gamma, 1.0]));

        let [r, g, b, a] = rgba.to_array();
        canvas::Color::Rgba(r, g, b, a)
    }
}

impl ToGammaColorSpace<U8RgbaPremultipliedPixel> for F32LinearPixel {
    #[inline]
    fn to_gamma_colorspace(&self, gamma: f64) -> U8RgbaPremultipliedPixel {
        static F32X4_255: Lazy<f32x4> = Lazy::new(|| f32x4::splat(255.0));

        // Remove gamma correction
        let gamma   = (1.0/gamma) as f32;
        let rgba    = self.0;
        let rgba    = rgba.min(f32x4::ONE).max(f32x4::ZERO);
        let rgba    = rgba.powf(gamma);
        let rgba    = rgba * *F32X4_255;
        let rgba    = rgba.fast_trunc_int();

        let [r, g, b, a] = rgba.to_array();
        U8RgbaPremultipliedPixel::from_components([r as _, g as _, b as _, a as _])
    }
}

impl AlphaBlend for F32LinearPixel {
    #[inline]
    fn alpha_blend_with_function(self, dest: Self, source_alpha_fn: AlphaFunction, dest_alpha_fn: AlphaFunction) -> Self {
        let src_alpha = self.alpha_component();
        let dst_alpha = dest.alpha_component();

        source_alpha_fn.apply(self, src_alpha, dst_alpha) + dest_alpha_fn.apply(dest, src_alpha, dst_alpha)
    }

    #[inline]
    fn multiply_alpha(self, factor: f64) -> Self {
        F32LinearPixel(self.0 * (factor as f32))
    }

    #[inline] fn source_over(self, dest: Self) -> Self        { let src_alpha = self.0.as_array_ref()[3]; F32LinearPixel(self.0 + dest.0*(1.0-src_alpha)) }
    #[inline] fn dest_over(self, dest: Self) -> Self          { let dst_alpha = dest.0.as_array_ref()[3]; F32LinearPixel(self.0*(1.0-dst_alpha) + dest.0) }
    #[inline] fn source_in(self, dest: Self) -> Self          { let dst_alpha = dest.0.as_array_ref()[3]; F32LinearPixel(self.0*dst_alpha) }
    #[inline] fn dest_in(self, dest: Self) -> Self            { let src_alpha = self.0.as_array_ref()[3]; F32LinearPixel(dest.0*src_alpha) }
    #[inline] fn source_held_out(self, dest: Self) -> Self    { let dst_alpha = dest.0.as_array_ref()[3]; F32LinearPixel(self.0*(1.0-dst_alpha)) }
    #[inline] fn dest_held_out(self, dest: Self) -> Self      { let src_alpha = self.0.as_array_ref()[3]; F32LinearPixel(dest.0*(1.0-src_alpha)) }
    #[inline] fn source_atop(self, dest: Self) -> Self        { self.alpha_blend(dest, AlphaOperation::SourceAtop) }
    #[inline] fn dest_atop(self, dest: Self) -> Self          { self.alpha_blend(dest, AlphaOperation::DestAtop) }
}

impl Add<F32LinearPixel> for F32LinearPixel {
    type Output=F32LinearPixel;

    #[inline]
    fn add(self, val: F32LinearPixel) -> F32LinearPixel {
        F32LinearPixel(self.0 + val.0)
    }
}

impl Add<f32> for F32LinearPixel {
    type Output=F32LinearPixel;

    #[inline]
    fn add(self, val: f32) -> F32LinearPixel {
        F32LinearPixel(self.0 + val)
    }
}

impl Sub<F32LinearPixel> for F32LinearPixel {
    type Output=F32LinearPixel;

    #[inline]
    fn sub(self, val: F32LinearPixel) -> F32LinearPixel {
        F32LinearPixel(self.0 - val.0)
    }
}

impl Sub<f32> for F32LinearPixel {
    type Output=F32LinearPixel;

    #[inline]
    fn sub(self, val: f32) -> F32LinearPixel {
        F32LinearPixel(self.0 - val)
    }
}

impl Mul<F32LinearPixel> for F32LinearPixel {
    type Output=F32LinearPixel;

    #[inline]
    fn mul(self, val: F32LinearPixel) -> F32LinearPixel {
        F32LinearPixel(self.0 * val.0)
    }
}

impl Mul<f32> for F32LinearPixel {
    type Output=F32LinearPixel;

    #[inline]
    fn mul(self, val: f32) -> F32LinearPixel {
        F32LinearPixel(self.0 * val)
    }
}

impl Div<F32LinearPixel> for F32LinearPixel {
    type Output=F32LinearPixel;

    #[inline]
    fn div(self, val: F32LinearPixel) -> F32LinearPixel {
        F32LinearPixel(self.0 / val.0)
    }
}

impl Div<f32> for F32LinearPixel {
    type Output=F32LinearPixel;

    #[inline]
    fn div(self, val: f32) -> F32LinearPixel {
        F32LinearPixel(self.0 / val)
    }
}

impl Neg for F32LinearPixel {
    type Output=F32LinearPixel;

    #[inline]
    fn neg(self) -> F32LinearPixel {
        F32LinearPixel(-self.0)
    }
}
