use super::solid_color::*;

use crate::pixel::*;
use crate::scanplan::*;

use std::marker::{PhantomData};
use std::ops::{Range};
use std::sync::*;

///
/// Pixel program that applies an alpha-blended colour using the source over algorithm
///
pub struct SourceOverColorProgram<TPixel: Copy + Send + Sync> { 
    pixel: PhantomData<Mutex<TPixel>>
}

impl<TPixel: Copy + Send + Sync> Default for SourceOverColorProgram<TPixel> 
where
    TPixel: AlphaBlend
{
    fn default() -> Self {
        SourceOverColorProgram { pixel: PhantomData }
    }
}

impl<TPixel: Copy + Send + Sync> PixelProgram for SourceOverColorProgram<TPixel>
where
    TPixel: AlphaBlend
{
    type Pixel          = TPixel;
    type ProgramData    = SolidColorData<TPixel>;

    #[inline]
    fn draw_pixels(&self, _data_cache: &PixelProgramRenderCache<Self::Pixel>, target: &mut [Self::Pixel], x_range: Range<i32>, _: &ScanlineTransform, _y_pos: f64, program_data: &Self::ProgramData) {
        for pixel in target[(x_range.start as usize)..(x_range.end as usize)].iter_mut() {
            *pixel = program_data.0.source_over(*pixel);
        }
    }
}
