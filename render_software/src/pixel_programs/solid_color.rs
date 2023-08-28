use crate::pixel::*;

use std::marker::{PhantomData};
use std::ops::{Range};
use std::sync::*;

/// Data for a solid colour pixel
pub struct SolidColorData<TPixel: Send + Sync>(pub TPixel);

///
/// Pixel program that writes out a solid colour according to the solid colour data it's supplied
///
pub struct SolidColorProgram<TPixel: Copy + Send + Sync> { 
    pixel: PhantomData<Mutex<TPixel>>
}

impl<TPixel: Copy + Send + Sync> Default for SolidColorProgram<TPixel> {
    fn default() -> Self {
        SolidColorProgram { pixel: PhantomData }
    }
}

impl<TPixel: Copy + Send + Sync> PixelProgram for SolidColorProgram<TPixel> {
    type Pixel          = TPixel;
    type ProgramData    = SolidColorData<TPixel>;

    #[inline]
    fn draw_pixels(&self, _data_cache: &PixelProgramDataCache<Self::Pixel>, target: &mut [Self::Pixel], x_range: Range<i32>, _y_pos: f64, program_data: &Self::ProgramData) {
        for pixel in target[(x_range.start as usize)..(x_range.end as usize)].iter_mut() {
            *pixel = program_data.0;
        }
    }
}
