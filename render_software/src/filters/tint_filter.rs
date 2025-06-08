use super::pixel_filter_trait::*;
use crate::pixel::*;

use flo_canvas as canvas;

use std::sync::*;

///
/// The tint blend filter
///
pub struct TintFilter<TPixel, const N: usize> 
where
    TPixel: Pixel<N>,
{
    tint: TPixel,
}

impl<TPixel, const N: usize> TintFilter<TPixel, N>
where
    TPixel: Pixel<N>,
{
    ///
    /// Creates a tint filter that will adjust the colours of its target by the specified amount
    ///
    pub fn with_color(color: canvas::Color, gamma: f64) -> Self {
        Self {
            tint: TPixel::from_color(color, gamma)
        }
    }
}

impl<TPixel, const N: usize> PixelFilter for TintFilter<TPixel, N>
where
    TPixel: Pixel<N>,
{
    type Pixel = TPixel;

    #[inline]
    fn with_scale(&self, _x_scale: f64, _y_scale: f64) -> Option<Arc<dyn Send + Sync + PixelFilter<Pixel=Self::Pixel>>> {
        None
    }

    #[inline]
    fn input_lines(&self) -> (usize, usize) {
        (0, 0)
    }

    #[inline]
    fn extra_columns(&self) -> (usize, usize) {
        (0, 0)
    }

    fn filter_line(&self, _ypos: usize, input_lines: &[&[Self::Pixel]], output_line: &mut [Self::Pixel]) {
        for (input, output) in input_lines[0].iter().zip(output_line.iter_mut()) {
            *output = *input * self.tint;
        }
    }
}
