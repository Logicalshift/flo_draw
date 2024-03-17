use super::pixel_filter_trait::*;
use crate::pixel::*;

use std::f64;

///
/// Computes the 1D weights for a gaussian blur for a particular standard deviation
///
fn weights_for_gaussian_blur(sigma: f64, step: f64, count: usize) -> Vec<f64> {
    // Short-circuit the case where count is 0
    if count == 0 { return vec![]; }

    let sigma_squared   = sigma * sigma;

    // Compute the weight at each position
    let uncorrected     = (0..count).into_iter()
        .map(|x| {
            let x = x as f64;
            let x = x * step;
            (1.0/((2.0*f64::consts::PI*sigma_squared).sqrt())) * (f64::consts::E.powf(-(x*x)/(2.0*sigma_squared)))
        })
        .collect::<Vec<_>>();

    // Correct the blur so that the weights all add up to 1
    let sum             = uncorrected[0] + uncorrected.iter().skip(1).fold(0.0, |x, y| x+*y)*2.0;
    let corrected       = uncorrected.into_iter().map(|weight| weight/sum).collect();

    corrected
}

///
/// Generates the weights for a gaussian blur with a particular radius
///
fn weights_for_radius(radius: f64) -> Vec<f64> {
    if radius <= 0.0 { return vec![1.0] }

    // Get the count for this radius
    let pixel_radius    = radius.ceil().max(1.0) as usize;
    let kernel_size     = ((pixel_radius-1)/2+1).max(1);

    let sigma   = 0.25;
    let step    = 1.0 / radius;

    weights_for_gaussian_blur(sigma, step, kernel_size)
}

///
/// Generates the weights for a gaussian blur with a particular radius, as pixel components
///
fn component_weights_for_radius<TPixel: Pixel<N>, const N: usize>(radius: f64) -> Box<[TPixel::Component]> {
    weights_for_radius(radius)
        .into_iter()
        .map(|val| TPixel::Component::with_value(val))
        .collect()
}

///
/// Filter that applies a one-dimensional kernel in the horizontal direction
///
pub struct HorizontalKernelFilter<TPixel, const N: usize>
where
    TPixel: Pixel<N>,
{
    /// Each pixel is multiplied by the values in the kernel, then summed. We only store half the kernel here, with the central pixel's proportion at the start
    kernel: Box<[TPixel::Component]>,
}

///
/// Filter that applies a one-dimensional kernel in the vertical direction
///
pub struct VerticalKernelFilter<TPixel, const N: usize>
where
    TPixel: Pixel<N>,
{
    /// Each pixel is multiplied by the values in the kernel, then summed. We only store half the kernel here, with the central pixel's proportion at the start
    kernel: Box<[TPixel::Component]>,
}

impl<TPixel, const N: usize> HorizontalKernelFilter<TPixel, N>
where
    TPixel: Pixel<N>,
{
    ///
    /// Creates a horizontal filter with a particular radius
    ///
    pub fn with_gaussian_blur_radius(radius: f64) -> Self {
        Self {
            kernel: component_weights_for_radius::<TPixel, N>(radius)
        }
    }
}

impl<TPixel, const N: usize> VerticalKernelFilter<TPixel, N>
where
    TPixel: Pixel<N>,
{
    ///
    /// Creates a horizontal filter with a particular radius
    ///
    pub fn with_gaussian_blur_radius(radius: f64) -> Self {
        Self {
            kernel: component_weights_for_radius::<TPixel, N>(radius)
        }
    }
}

impl<TPixel, const N: usize> PixelFilter for HorizontalKernelFilter<TPixel, N>
where
    TPixel: Pixel<N>,
{
    type Pixel = TPixel;

    #[inline]
    fn input_lines(&self) -> (usize, usize) {
        (0, 0)
    }

    #[inline]
    fn extra_columns(&self) -> (usize, usize) {
        (self.kernel.len()-1, self.kernel.len()-1)
    }

    fn filter_line(&self, _ypos: usize, input_lines: &[&[Self::Pixel]], output_line: &mut [Self::Pixel]) {
        let input_line  = input_lines[0];
        let kernel      = &self.kernel;
        let kernel_len  = kernel.len();

        for idx in (kernel_len-1)..(input_line.len()-(kernel_len-1)) {
            let mut pixel = input_line[idx] * kernel[0];

            for kern_idx in 1..kernel_len {
                let kernel_val = kernel[kern_idx];

                pixel = pixel + (input_line[idx + kern_idx] * kernel_val) + (input_line[idx - kern_idx] * kernel_val);
            }

            output_line[idx - (kernel_len-1)] = pixel;
        }
    }
}

impl<TPixel, const N: usize> PixelFilter for VerticalKernelFilter<TPixel, N>
where
    TPixel: Pixel<N>,
{
    type Pixel = TPixel;

    fn input_lines(&self) -> (usize, usize) {
        (self.kernel.len()-1, self.kernel.len()-1)
    }

    fn extra_columns(&self) -> (usize, usize) {
        (0, 0)
    }

    fn filter_line(&self, _ypos: usize, input_lines: &[&[Self::Pixel]], output_line: &mut [Self::Pixel]) {
        let kernel      = &self.kernel;
        let kernel_len  = kernel.len();
        let mid_pos     = kernel_len-1;

        for idx in 0..output_line.len() {
            let mut pixel = input_lines[mid_pos][idx] * kernel[0];

            for kern_idx in 1..kernel_len {
                let kernel_val = kernel[kern_idx];

                pixel = pixel + (input_lines[mid_pos+kern_idx][idx] * kernel_val) + (input_lines[mid_pos-kern_idx][idx] * kernel_val);
            }

            output_line[idx] = pixel;
        }
    }
}
