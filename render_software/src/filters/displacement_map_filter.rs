use super::pixel_filter_trait::*;
use crate::pixel::*;

use std::sync::*;
use std::marker::{PhantomData};

///
/// A displacement map filter reads from a target texture and displaces each pixel by a specific amount 
///
pub struct DisplacementMapFilter<TPixel, const N: usize>
where
    TPixel: Pixel<N>,
{
    displacement_map:   Arc<U16LinearTexture>,
    offset_x:           f64,
    offset_y:           f64,
    pixel:              PhantomData<TPixel>,
}

impl<TPixel, const N: usize> DisplacementMapFilter<TPixel, N>
where
    TPixel: Pixel<N>,
{
    ///
    /// Creates a new displacement map filter
    ///
    /// The offsets here are the maximum value in pixels that the image can move away from its original value
    ///
    pub fn with_displacement_map(map: &Arc<U16LinearTexture>, offset_x: f64, offset_y: f64) -> Self {
        DisplacementMapFilter {
            displacement_map:   Arc::clone(map),
            offset_x:           offset_x,
            offset_y:           offset_y,
            pixel:              PhantomData,
        }
    }
}

impl<TPixel, const N: usize> PixelFilter for DisplacementMapFilter<TPixel, N>
where
    TPixel: Pixel<N>,
{
    type Pixel = TPixel;

    fn input_lines(&self) -> (usize, usize) {
        (self.offset_y.ceil() as usize, self.offset_y.ceil() as usize)
    }

    fn extra_columns(&self) -> (usize, usize) {
        (self.offset_x.ceil() as usize, self.offset_x.ceil() as usize)
    }

    fn filter_line(&self, input_lines: &[&[Self::Pixel]], output_line: &mut [Self::Pixel]) {
        // Read a line from the displacement map
        todo!()
    }
}