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
    TPixel: Send + Sync + Copy + AlphaBlend,
{
    type Pixel = TPixel;

    type ProgramData = GradientData<TPixel>;

    fn draw_pixels(&self, _data_cache: &PixelProgramRenderCache<Self::Pixel>, target: &mut [Self::Pixel], x_range: std::ops::Range<i32>, x_transform: &crate::scanplan::ScanlineTransform, y_pos: f64, program_data: &Self::ProgramData) {
        let gradient    = &*program_data.gradient;
        let alpha       = TPixel::Component::with_value(program_data.alpha);

        // Compute the start and end x of the gradient range
        let t       = &program_data.transform[0];
        let start_x = x_transform.pixel_x_to_source_x(x_range.start);
        let end_x   = x_transform.pixel_x_to_source_x(x_range.end);

        let start_x = t[0] * start_x + t[1] * y_pos + t[2];
        let end_x   = t[0] * end_x * t[1] * y_pos * t[2];
        let step    = (end_x - start_x) / (x_range.len() as f64);

        let max_x   = (program_data.gradient.len()-1) as f64;

        for (target_x, target) in x_range.clone().zip(target[(x_range.start as usize)..(x_range.end as usize)].iter_mut()) {
            // Read two pixels from the gradient and interpolate them
            let xpos    = target_x as f64;
            let x1      = start_x + xpos * step;
            let x2      = x1 + 1.0;
            let x1      = x1.max(0.0).min(max_x);
            let x2      = x2.max(0.0).min(max_x);

            let fract   = x1.fract();
            let x1      = x1 as usize;
            let x2      = x2 as usize;

            let px1     = gradient[x1];
            let px2     = gradient[x2];

            // Interpolate to generate the final pixel
            let fract = TPixel::Component::with_value(fract);
            *target = (px1 * fract + px2 * (TPixel::Component::one() - fract)) * alpha;
        }
    }
}
