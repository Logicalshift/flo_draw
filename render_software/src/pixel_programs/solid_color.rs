use crate::pixel::*;

use std::ops::{Range};

/// Data for a solid colour pixel
pub struct SolidColorData(pub F32LinearPixel);

///
/// Pixel program that writes out a solid colour according to the solid colour data it's supplied
///
pub struct SolidColorProgram;

impl PixelProgram for SolidColorProgram {
    type Pixel          = F32LinearPixel;
    type ProgramData    = SolidColorData;

    #[inline]
    fn draw_pixels(&self, _data_cache: &PixelProgramDataCache<Self::Pixel>, target: &mut [Self::Pixel], x_range: Range<i32>, _y_pos: f64, program_data: &Self::ProgramData) {
        for pixel in target[(x_range.start as usize)..(x_range.end as usize)].iter_mut() {
            *pixel = program_data.0;
        }
    }
}
