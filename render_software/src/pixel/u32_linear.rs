use super::alpha_blend_trait::*;
use super::gamma_lut::*;
use super::pixel_trait::*;
use super::to_gamma_colorspace_trait::*;
use super::u32_fixed_point::*;
use super::u8_rgba::*;

use flo_canvas as canvas;

use wide::*;

use std::ops::*;
use std::cell::{RefCell};

///
/// A pixel using linear fixed-point components, with the alpha value pre-multiplied (16 bits per channel)
///
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct U32LinearPixel(u32x4);

impl Default for U32LinearPixel {
    #[inline]
    fn default() -> Self {
        U32LinearPixel(u32x4::splat(0))
    }
}

impl Pixel<4> for U32LinearPixel {
    type Component = U32FixedPoint;

    #[inline]
    fn black() -> U32LinearPixel {
        U32LinearPixel(u32x4::new([0, 0, 0, 65535]))
    }

    #[inline]
    fn white() -> U32LinearPixel {
        U32LinearPixel(u32x4::new([65535, 65535, 65535, 65535]))
    }

    #[inline]
    fn from_components(components: [Self::Component; 4]) -> Self {
        U32LinearPixel(u32x4::new(U32FixedPoint::to_u32_slice(components)))
    }

    #[inline]
    fn to_components(&self) -> [Self::Component; 4] {
        U32FixedPoint::from_u32_slice(self.0.to_array())
    }

    #[inline]
    fn get(&self, component: usize) -> Self::Component { 
        self.to_components()[component] 
    }

    #[inline]
    fn alpha_component(&self) -> Self::Component {
        U32FixedPoint(self.0.as_array_ref()[3])
    }

    #[inline]
    fn from_color(color: canvas::Color, gamma: f64) -> Self {
        let (r, g, b, a) = color.to_rgba_components();

        // Add premultiplication and gamma correction
        let gamma = gamma as f32;
        let pixel = f32x4::new([r, g, b, a]);
        let pixel = pixel.pow_f32x4(f32x4::new([gamma, gamma, gamma, 1.0]));
        let pixel = pixel * f32x4::new([a, a, a, 1.0]);
        let pixel = pixel * 65535.0;
        let pixel = pixel.fast_round_int();
        let pixel = pixel.to_array();

        U32LinearPixel(u32x4::new([pixel[0] as _, pixel[1] as _, pixel[2] as _, pixel[3] as _]))
    }

    #[inline]
    fn to_color(&self, gamma: f64) -> canvas::Color {
        let alpha   = self.0.as_array_ref()[3];
        let alpha   = (alpha as f32)/65535.0;

        // Remove premultiplication and gamma correction
        let gamma       = (1.0/gamma) as f32;
        let components  = self.0.as_array_ref();
        let rgba        = f32x4::from([components[0] as f32, components[1] as f32, components[2] as f32, components[3] as f32]);
        let rgba        = rgba / 65535.0;
        let rgba        = rgba / f32x4::new([alpha, alpha, alpha, 1.0]);
        let rgba        = rgba.pow_f32x4(f32x4::new([gamma, gamma, gamma, 1.0]));
    
        let [r, g, b, a] = rgba.to_array();
        canvas::Color::Rgba(r, g, b, a)
    }
}

impl ToGammaColorSpace<U8RgbaPremultipliedPixel> for U32LinearPixel {
    fn to_gamma_colorspace(input_pixels: &[U32LinearPixel], output_pixels: &mut [U8RgbaPremultipliedPixel], gamma: f64) {
        thread_local! {
            // The gamma-correction look-up table is generated once per thread, saves us doing the expensive 'powf()' operation
            pub static GAMMA_LUT: RefCell<U8GammaLut> = RefCell::new(U8GammaLut::new(1.0/2.2));
        }

        GAMMA_LUT.with(move |gamma_lut| {
            // This isn't re-entrant so only this function can use the gamma-correction table 
            let mut gamma_lut = gamma_lut.borrow_mut();

            // Update the LUT if needed (should be rare, we'll generally be working on converting a whole frame at once)
            let gamma = 1.0/gamma;
            if gamma != gamma_lut.gamma() { *gamma_lut = U8GammaLut::new(gamma) };

            // Some values we use during the conversion
            let f32x4_65535 = f32x4::splat(65535.0);

            let mut input   = input_pixels.iter();
            let mut output  = output_pixels.iter_mut();

            while let (Some(input), Some(output)) = (input.next(), output.next()) {
                // Convert the pixel to u8 format and apply gamma correction
                let rgba    = input.0;

                // This produces SRGB format, where the values are pre-multiplied before gamma correction
                let [r, g, b, a] = rgba.to_array();
                *output = U8RgbaPremultipliedPixel::from_components([
                    gamma_lut.look_up(r as _), 
                    gamma_lut.look_up(g as _), 
                    gamma_lut.look_up(b as _), 
                    (a >> 8) as u8]);
            }
        })
    }
}

impl AlphaBlend for U32LinearPixel {
    #[inline]
    fn alpha_blend_with_function(self, dest: Self, source_alpha_fn: AlphaFunction, dest_alpha_fn: AlphaFunction) -> Self {
        let src_alpha = self.alpha_component();
        let dst_alpha = dest.alpha_component();

        source_alpha_fn.apply(self, src_alpha, dst_alpha) + dest_alpha_fn.apply(dest, src_alpha, dst_alpha)
    }

    #[inline]
    fn multiply_alpha(self, factor: f64) -> Self {
        self * (factor as f32)
    }

    #[inline] fn source_over(self, dest: Self) -> Self        { let src_alpha = self.0.as_array_ref()[3]; U32LinearPixel(self.0 + dest.0*(65535-src_alpha)) }
    #[inline] fn dest_over(self, dest: Self) -> Self          { let dst_alpha = dest.0.as_array_ref()[3]; U32LinearPixel(self.0*(65535-dst_alpha) + dest.0) }
    #[inline] fn source_in(self, dest: Self) -> Self          { let dst_alpha = dest.0.as_array_ref()[3]; U32LinearPixel(self.0*dst_alpha) }
    #[inline] fn dest_in(self, dest: Self) -> Self            { let src_alpha = self.0.as_array_ref()[3]; U32LinearPixel(dest.0*src_alpha) }
    #[inline] fn source_held_out(self, dest: Self) -> Self    { let dst_alpha = dest.0.as_array_ref()[3]; U32LinearPixel(self.0*(65535-dst_alpha)) }
    #[inline] fn dest_held_out(self, dest: Self) -> Self      { let src_alpha = self.0.as_array_ref()[3]; U32LinearPixel(dest.0*(65535-src_alpha)) }
    #[inline] fn source_atop(self, dest: Self) -> Self        { self.alpha_blend(dest, AlphaOperation::SourceAtop) }
    #[inline] fn dest_atop(self, dest: Self) -> Self          { self.alpha_blend(dest, AlphaOperation::DestAtop) }
}

impl Add<U32LinearPixel> for U32LinearPixel {
    type Output=U32LinearPixel;

    #[inline]
    fn add(self, val: U32LinearPixel) -> U32LinearPixel {
        U32LinearPixel(self.0 + val.0)
    }
}

impl Add<U32FixedPoint> for U32LinearPixel {
    type Output=U32LinearPixel;

    #[inline]
    fn add(self, val: U32FixedPoint) -> U32LinearPixel {
        U32LinearPixel(self.0 + val.0)
    }
}

impl Sub<U32LinearPixel> for U32LinearPixel {
    type Output=U32LinearPixel;

    #[inline]
    fn sub(self, val: U32LinearPixel) -> U32LinearPixel {
        U32LinearPixel(self.0 - val.0)
    }
}

impl Sub<U32FixedPoint> for U32LinearPixel {
    type Output=U32LinearPixel;

    #[inline]
    fn sub(self, val: U32FixedPoint) -> U32LinearPixel {
        U32LinearPixel(self.0 - val.0)
    }
}

impl Mul<U32LinearPixel> for U32LinearPixel {
    type Output=U32LinearPixel;

    #[inline]
    fn mul(self, val: U32LinearPixel) -> U32LinearPixel {
        U32LinearPixel((self.0 * val.0) >> 16)
    }
}

impl Mul<U32FixedPoint> for U32LinearPixel {
    type Output=U32LinearPixel;

    #[inline]
    fn mul(self, val: U32FixedPoint) -> U32LinearPixel {
        U32LinearPixel((self.0 * val.0) >> 16)
    }
}

impl Mul<f32> for U32LinearPixel {
    type Output=U32LinearPixel;

    #[inline]
    fn mul(self, val: f32) -> U32LinearPixel {
        let val = (val * 65535.0) as u32;

        U32LinearPixel((self.0 * val) >> 16)
    }
}

impl Div<U32LinearPixel> for U32LinearPixel {
    type Output=U32LinearPixel;

    #[inline]
    fn div(self, val: U32LinearPixel) -> U32LinearPixel {
        let shifted: u32x4  = self.0 << 16;
        let components      = shifted.to_array();
        let val_components  = val.0.to_array();

        U32LinearPixel([
            components[0] / val_components[0],
            components[1] / val_components[1],
            components[2] / val_components[2],
            components[3] / val_components[3],
        ].into())
    }
}

impl Div<U32FixedPoint> for U32LinearPixel {
    type Output=U32LinearPixel;

    #[inline]
    fn div(self, val: U32FixedPoint) -> U32LinearPixel {
        let shifted: u32x4  = self.0 << 16;
        let components      = shifted.to_array();

        U32LinearPixel([
            components[0] / val.0,
            components[1] / val.0,
            components[2] / val.0,
            components[3] / val.0,
        ].into())
    }
}

impl Div<f32> for U32LinearPixel {
    type Output=U32LinearPixel;

    #[inline]
    fn div(self, val: f32) -> U32LinearPixel {
        let val             = (val * 65535.0) as u32;
        let shifted: u32x4  = self.0 << 16;
        let components      = shifted.to_array();

        U32LinearPixel([
            components[0] / val,
            components[1] / val,
            components[2] / val,
            components[3] / val,
        ].into())
    }
}

impl Neg for U32LinearPixel {
    type Output=U32LinearPixel;

    #[inline]
    fn neg(self) -> U32LinearPixel {
        U32LinearPixel(-self.0)
    }
}
