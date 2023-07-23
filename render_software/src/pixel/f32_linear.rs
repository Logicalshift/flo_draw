use super::alpha_blend_trait::*;
use super::pixel_trait::*;
use super::u8_rgba::*;

use flo_canvas as canvas;

use wide::*;

use std::ops::*;

///
/// A pixel using linear floating-point components
///
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct F32LinearPixel(f32x4);

impl Pixel<4> for F32LinearPixel {
    type Component = f32;

    #[inline]
    fn black() -> F32LinearPixel {
        F32LinearPixel(f32x4::new([0.0, 0.0, 0.0, 1.0]))
    }

    #[inline]
    fn white() -> F32LinearPixel {
        F32LinearPixel(f32x4::new([0.0, 0.0, 0.0, 1.0]))
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

        let gamma = gamma as f32;
        let pixel = f32x4::new([r, g, b, a]);
        let pixel = pixel.pow_f32x4(f32x4::new([gamma, gamma, gamma, 1.0]));

        F32LinearPixel(pixel)
    }

    #[inline]
    fn to_color(&self, gamma: f64) -> canvas::Color {
        let gamma   = (1.0/gamma) as f32;
        let rgba    = self.0.pow_f32x4(f32x4::new([gamma, gamma, gamma, 1.0]));

        let [r, g, b, a] = rgba.to_array();
        canvas::Color::Rgba(r, g, b, a)
    }

    #[inline]
    fn to_u8_rgba(&self, gamma: f64) -> U8RgbaPremultipliedPixel {
        let gamma   = (1.0/gamma) as f32;
        let rgba    = self.0.pow_f32x4(f32x4::new([gamma, gamma, gamma, 1.0]));
        let rgba    = rgba * 255.0;
        let rgba    = rgba.fast_trunc_int();
        let rgba    = rgba.min(i32x4::splat(255)).max(i32x4::splat(0));

        let [r, g, b, a] = rgba.to_array();
        U8RgbaPremultipliedPixel::from_components([r as _, g as _, b as _, a as _])
    }
}

impl AlphaBlend for F32LinearPixel {
    #[inline]
    fn alpha_blend_with_function(self, dest: Self, source_alpha: AlphaFunction, dest_alpha: AlphaFunction) -> Self {
        let src_alpha = self.alpha_component();
        let dst_alpha = dest.alpha_component();

        source_alpha.apply(self, src_alpha, dst_alpha) + dest_alpha.apply(dest, src_alpha, dst_alpha)
    }
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
