use super::alpha_blend_trait::*;
use super::to_gamma_colorspace_trait::*;
use super::to_linear_colorspace_trait::*;
use super::pixel_trait::*;
use super::u8_rgba::*;
use super::u16_rgba::*;
use super::u32_argb::*;
use super::gamma_lut::*;

use flo_canvas as canvas;

use wide::*;
use once_cell::sync::{Lazy};

use std::cell::{RefCell};
use std::collections::{HashMap};
use std::ops::*;
use std::sync::*;

///
/// A pixel using linear floating-point components, with the alpha value pre-multiplied
///
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct F32CmykaPixel(f32x4, f32);

impl F32CmykaPixel {
	#[inline]
	pub fn to_cmyk(&self) -> [f32; 4] {
		self.0.to_array()
	}
	#[inline]
	pub fn to_u8(&self) -> [u8; 5] {
		let a = self.1;
		let [c, m, y, k] = *self.0.div(a).as_array_ref();
		[c, m, y, k, a].map(|f| (f * 255.0).round() as u8)
	}
}

impl Default for F32CmykaPixel {
	#[inline]
	fn default() -> Self {
		F32CmykaPixel(f32x4::splat(0.0), 0.0)
	}
}

impl Pixel<5> for F32CmykaPixel {
	#[inline]
	fn black() -> F32CmykaPixel {
		F32CmykaPixel(f32x4::new([0.0, 0.0, 0.0, 1.0]), 1.0)
	}
	
	#[inline]
	fn white() -> F32CmykaPixel {
		F32CmykaPixel(f32x4::new([0.0, 0.0, 0.0, 0.0]), 1.0)
	}
	
	#[inline]
	fn from_components(components: [Self::Component; 5]) -> Self {
		let [c, m, y, k, a] = components;
		F32CmykaPixel(f32x4::new([c, m, y, k]), a)
	}
	
	#[inline]
	fn to_components(&self) -> [Self::Component; 5] {
		let mut arr = [0.0; 5];
		arr[0..4].copy_from_slice(self.0.as_array_ref());
		arr[4] = self.1;
		arr
	}
	
	#[inline]
	fn get(&self, component: usize) -> Self::Component {
		match component {
			4 => self.1,
			_ => self.0.as_array_ref()[component]
		}
	}
	
	#[inline]
	fn from_color(color: canvas::Color, _gamma: f64) -> Self {
		let (c, m, y, k, a) = color.to_cmyka_components();
		let cmyk = f32x4::new([c, m, y, k]);
		F32CmykaPixel(cmyk * a, a)
	}
	
	#[inline]
	fn to_color(&self, _gamma: f64) -> canvas::Color {
		let a = self.1;
		let [c, m, y, k] = *self.0.div(a).as_array_ref();
		canvas::Color::Cmyka(c, m, y, k, a)
	}
}

// Lookup tables for gamma values
// static GAMMA_LUT: Lazy<Mutex<HashMap<i64, Arc<U8GammaLut>>>> = Lazy::new(|| Mutex::new(HashMap::new()));

impl ToGammaColorSpace<U8RgbaPremultipliedPixel> for F32CmykaPixel {
	fn to_gamma_colorspace(input_pixels: &[F32CmykaPixel], output_pixels: &mut [U8RgbaPremultipliedPixel], gamma: f64) {
		todo!("idk")
	}
}

impl ToGammaColorSpace<U32ArgbPremultipliedPixel> for F32CmykaPixel {
	fn to_gamma_colorspace(input_pixels: &[F32CmykaPixel], output_pixels: &mut [U32ArgbPremultipliedPixel], gamma: f64) {
		todo!("idk")
	}
}

impl ToLinearColorSpace<U16LinearPixel> for F32CmykaPixel {
	fn to_linear_colorspace(input_pixels: &[Self], output_pixels: &mut [U16LinearPixel]) {
		todo!("idk")
	}
}

impl AlphaBlend for F32CmykaPixel {
	type Component = f32;
	
	#[inline]
	fn alpha_blend_with_function(self, dest: Self, source_alpha_fn: AlphaFunction, dest_alpha_fn: AlphaFunction) -> Self {
		let src_alpha = self.alpha_component();
		let dst_alpha = dest.alpha_component();
		
		source_alpha_fn.apply(self, src_alpha, dst_alpha) + dest_alpha_fn.apply(dest, src_alpha, dst_alpha)
	}
	
	#[inline]
	fn alpha_component(&self) -> Self::Component {
		self.1
	}
	
	#[inline]
	fn multiply_alpha(self, factor: f64) -> Self {
		self * (factor as f32)
	}
	
	#[inline] fn source_over(self, dest: Self) -> Self        { let src_alpha = self.1; self + dest * (1.0-src_alpha) }
	#[inline] fn dest_over(self, dest: Self) -> Self          { let dst_alpha = dest.1; self * (1.0-dst_alpha) + dest }
	#[inline] fn source_in(self, dest: Self) -> Self          { let dst_alpha = dest.1; self * dst_alpha }
	#[inline] fn dest_in(self, dest: Self) -> Self            { let src_alpha = self.1; dest * src_alpha }
	#[inline] fn source_held_out(self, dest: Self) -> Self    { let dst_alpha = dest.1; self*(1.0-dst_alpha) }
	#[inline] fn dest_held_out(self, dest: Self) -> Self      { let src_alpha = self.1; dest*(1.0-src_alpha) }
	#[inline] fn source_atop(self, dest: Self) -> Self        { self.alpha_blend(dest, AlphaOperation::SourceAtop) }
	#[inline] fn dest_atop(self, dest: Self) -> Self          { self.alpha_blend(dest, AlphaOperation::DestAtop) }
}

impl Add<F32CmykaPixel> for F32CmykaPixel {
	type Output= F32CmykaPixel;
	
	#[inline]
	fn add(self, val: F32CmykaPixel) -> F32CmykaPixel {
		F32CmykaPixel(self.0 + val.0, self.1 + val.1)
	}
}

impl Add<f32> for F32CmykaPixel {
	type Output= F32CmykaPixel;
	
	#[inline]
	fn add(self, val: f32) -> F32CmykaPixel {
		F32CmykaPixel(self.0 + val, self.1 + val)
	}
}

impl Sub<F32CmykaPixel> for F32CmykaPixel {
	type Output= F32CmykaPixel;
	
	#[inline]
	fn sub(self, val: F32CmykaPixel) -> F32CmykaPixel {
		F32CmykaPixel(self.0 - val.0, self.1 - val.1)
	}
}

impl Sub<f32> for F32CmykaPixel {
	type Output= F32CmykaPixel;
	
	#[inline]
	fn sub(self, val: f32) -> F32CmykaPixel {
		F32CmykaPixel(self.0 - val, self.1 - val)
	}
}

impl Mul<F32CmykaPixel> for F32CmykaPixel {
	type Output= F32CmykaPixel;
	
	#[inline]
	fn mul(self, val: F32CmykaPixel) -> F32CmykaPixel {
		F32CmykaPixel(self.0 * val.0, self.1 * val.1)
	}
}

impl Mul<f32> for F32CmykaPixel {
	type Output= F32CmykaPixel;
	
	#[inline]
	fn mul(self, val: f32) -> F32CmykaPixel {
		F32CmykaPixel(self.0 * val, self.1 * val)
	}
}

impl Div<F32CmykaPixel> for F32CmykaPixel {
	type Output= F32CmykaPixel;
	
	#[inline]
	fn div(self, val: F32CmykaPixel) -> F32CmykaPixel {
		F32CmykaPixel(self.0 / val.0, self.1 / val.1)
	}
}

impl Div<f32> for F32CmykaPixel {
	type Output= F32CmykaPixel;
	
	#[inline]
	fn div(self, val: f32) -> F32CmykaPixel {
		F32CmykaPixel(self.0 / val, self.1 / val)
	}
}

impl Neg for F32CmykaPixel {
	type Output= F32CmykaPixel;
	
	#[inline]
	fn neg(self) -> F32CmykaPixel {
		F32CmykaPixel(-self.0, -self.1)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	
	#[test]
	fn it_works() {
	}
}
