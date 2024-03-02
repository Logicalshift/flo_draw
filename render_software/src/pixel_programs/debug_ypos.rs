use crate::pixel::*;
use crate::scanplan::*;

use flo_canvas::*;

use std::marker::{PhantomData};

///
/// Debugging program that outputs a colour that indicates the y position used to generate a particular scanline
///
/// This can be used to get the parameters needed to reproduce buggy scanline generation
///
pub struct DebugYposProgram<TPixel, const N: usize> {
    pixel: PhantomData<TPixel>
}

impl<TPixel, const N: usize> PixelProgram for DebugYposProgram<TPixel, N>
where
    TPixel: Pixel<N>,
{
    type Pixel          = TPixel;
    type ProgramData    = f64;

    fn draw_pixels(&self, _: &PixelProgramRenderCache<Self::Pixel>, target: &mut [Self::Pixel], x_range: std::ops::Range<i32>, _: &ScanlineTransform, y_pos: f64, multiplier: &f64) {
        // Use the multiplier to calculate the y-position
        let y_pos = y_pos * *multiplier;

        // Calculate RGB values for the y position (gives us a 0-100 range from all three colour components)
        let r = y_pos % 1.0;
        let g = ((y_pos - r) / 10.0) % 1.0;
        let b = ((((y_pos - r) / 10.0) - g) / 10.0) % 1.0;

        // Fill the target range with the specified pixel colour
        let pixel = TPixel::from_color(Color::Rgba(r as _, g as _, b as _, 1.0), 2.2);

        target[(x_range.start as usize)..(x_range.end as usize)].iter_mut()
            .for_each(|target_pixel| *target_pixel = pixel);
    }
}
