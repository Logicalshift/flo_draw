use super::pixel_filter_trait::*;
use crate::pixel::*;

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

    fn filter_line(&self, input_lines: &[&[Self::Pixel]], output_line: &mut [Self::Pixel]) {
        let input_line  = input_lines[0];
        let kernel      = &self.kernel;
        let kernel_len  = kernel.len();

        for idx in (kernel_len-1)..(input_line.len()-(kernel_len-1)) {
            let mut pixel = input_line[idx] * kernel[0];

            for kern_idx in 1..kernel_len {
                let kernel_val = kernel[kern_idx];

                pixel = pixel + (input_line[idx + kern_idx] * kernel_val) + (input_line[idx - kern_idx] * kernel_val);
            }

            output_line[idx - kernel_len] = pixel;
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

    fn filter_line(&self, input_lines: &[&[Self::Pixel]], output_line: &mut [Self::Pixel]) {
        let kernel      = &self.kernel;
        let kernel_len  = kernel.len();
        let mid_pos     = kernel_len-1;

        for idx in 0..output_line.len() {
            let mut pixel = input_lines[mid_pos][idx] * kernel[0];

            for kern_idx in 1..kernel_len {
                let kernel_val = kernel[kern_idx];

                pixel = pixel + (input_lines[mid_pos+kern_idx][idx] * kernel_val) + (input_lines[mid_pos-kern_idx][idx] * kernel_val);
            }

            output_line[idx - kernel_len] = pixel;
        }
    }
}
