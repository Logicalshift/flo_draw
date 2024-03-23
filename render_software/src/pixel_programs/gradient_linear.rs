use crate::pixel::*;

use std::marker::{PhantomData};
use std::sync::*;

///
/// Data for the gradient programs
///
pub struct GradientData<TPixel> {
    /// The texture that this program will read from (effectively a single dimensional texture)
    pub (crate) gradient: Arc<Vec<TPixel>>,

    /// Alpha value to multiply the gradient pixel values by
    pub (crate) alpha: f64,

    // The top two rows of the transformation matrix between source coordinates and gradient coordinates
    pub (crate) transform: [[f64; 3]; 2],
}

///
/// A pixel program that generates a linear gradient 
///
pub struct LinearGradientProgram<TPixel>
where
    TPixel: AlphaBlend,
{
    pixel: PhantomData<TPixel>
}

impl<TPixel> Default for LinearGradientProgram<TPixel>
where
    TPixel: AlphaBlend,
{
    fn default() -> Self {
        LinearGradientProgram { 
            pixel: PhantomData
        }
    }
}

impl<TPixel> PixelProgram for LinearGradientProgram<TPixel>
where
    TPixel: Send + Sync + AlphaBlend,
{
    type Pixel = TPixel;

    type ProgramData = GradientData<TPixel>;

    fn draw_pixels(&self, data_cache: &PixelProgramRenderCache<Self::Pixel>, target: &mut [Self::Pixel], x_range: std::ops::Range<i32>, x_transform: &crate::scanplan::ScanlineTransform, y_pos: f64, program_data: &Self::ProgramData) {
        todo!()
    }
}
