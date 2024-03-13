use super::pixel_filter_trait::*;
use crate::pixel::*;

///
/// The alpha blend filter
///
pub struct AlphaBlendFilter<TPixel, const N: usize> 
where
    TPixel: Pixel<N>,
{
    alpha: TPixel::Component,
}

impl<TPixel, const N: usize> AlphaBlendFilter<TPixel, N>
where
    TPixel: Pixel<N>,
{
    ///
    /// Creates an alpha blend filter that will adjust the alpha value of its target by the specified amouint
    ///
    pub fn with_alpha(alpha: f64) -> Self {
        Self {
            alpha: TPixel::Component::with_value(alpha)
        }
    }
}

impl<TPixel, const N: usize> PixelFilter for AlphaBlendFilter<TPixel, N>
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
        (0, 0)
    }

    fn filter_line(&self, input_lines: &[&[Self::Pixel]], output_line: &mut [Self::Pixel]) {
        for (input, output) in input_lines[0].iter().zip(output_line.iter_mut()) {
            *output = *input * self.alpha;
        }
    }
}
