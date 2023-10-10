use crate::pixel::*;
use crate::scanplan::*;

use std::marker::{PhantomData};
use std::ops::{Range};
use std::sync::*;

/// Data for a pixel drawn using a blending algorithm
pub struct BlendColorData<TPixel: Send + Sync>(pub AlphaOperation, pub TPixel);

///
/// Pixel program that applies an alpha-blended colour using the source over algorithm
///
pub struct BlendColorProgram<TPixel: Copy + Send + Sync> { 
    pixel: PhantomData<Mutex<TPixel>>
}

impl<TPixel: Copy + Send + Sync> Default for BlendColorProgram<TPixel> 
where
    TPixel: AlphaBlend
{
    fn default() -> Self {
        BlendColorProgram { pixel: PhantomData }
    }
}

impl<TPixel: Copy + Send + Sync> PixelProgram for BlendColorProgram<TPixel>
where
    TPixel: AlphaBlend
{
    type Pixel          = TPixel;
    type ProgramData    = BlendColorData<TPixel>;

    #[inline]
    fn draw_pixels(&self, _data_cache: &PixelProgramDataCache<Self::Pixel>, target: &mut [Self::Pixel], x_range: Range<i32>, _: &ScanlineTransform, _y_pos: f64, program_data: &Self::ProgramData) {
        let op = program_data.0.get_function::<TPixel>();

        for pixel in target[(x_range.start as usize)..(x_range.end as usize)].iter_mut() {
            *pixel = op(program_data.1, *pixel);
        }
    }
}
