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
    TPixel: AlphaBlend,
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
    fn draw_pixels(&self, _data_cache: &PixelProgramRenderCache<Self::Pixel>, target: &mut [Self::Pixel], x_range: Range<i32>, _: &ScanlineTransform, _y_pos: f64, program_data: &Self::ProgramData) {
        let op = program_data.0.get_function::<TPixel>();

        for pixel in target[(x_range.start as usize)..(x_range.end as usize)].iter_mut() {
            *pixel = op(program_data.1, *pixel);
        }
    }
}

///
/// The data used for a blending program
///
pub struct BlendRenderingData<TProgram: PixelProgram>(AlphaOperation, f64, TProgram::ProgramData);

///
/// Renders the result of another pixel program, then applies a blending function to it
///
pub struct BlendRenderingProgram<TProgram: PixelProgram> {
    /// The program that this rendering program will run
    program: TProgram,    
}

impl<TProgram: PixelProgram> BlendRenderingProgram<TProgram> {
    ///
    /// Creates a new blending program
    ///
    #[inline]
    pub fn new(program: TProgram) -> Self {
        BlendRenderingProgram { program }
    }
}

impl<TProgram> PixelProgram for BlendRenderingProgram<TProgram> 
where
    TProgram:           PixelProgram,
    TProgram::Pixel:    Copy + AlphaBlend + Default,
{
    type Pixel          = TProgram::Pixel;
    type ProgramData    = BlendRenderingData<TProgram>;

    #[inline]
    fn draw_pixels(&self, data_cache: &PixelProgramRenderCache<Self::Pixel>, target: &mut [Self::Pixel], x_range: Range<i32>, x_transform: &ScanlineTransform, y_pos: f64, program_data: &Self::ProgramData) {
        // Prepare to render by fetching the alpha operation
        let BlendRenderingData(op, transparency, program_data) = program_data;
        let transparency    = <TProgram::Pixel as AlphaBlend>::Component::with_value(*transparency);
        let op              = op.get_function::<TProgram::Pixel>();

        // Render the source pixels in a separate buffer
        let length              = (x_range.end - x_range.start) as usize;
        let mut source_pixels   = vec![TProgram::Pixel::default(); length];

        self.program.draw_pixels(data_cache, &mut source_pixels, x_range.clone(), x_transform, y_pos, program_data);

        // Blend with the target pixels
        for (target_pixel, source_pixel) in target[(x_range.start as usize)..(x_range.end as usize)].iter_mut().zip(source_pixels.into_iter()) {
            let source_pixel = source_pixel * transparency;

            *target_pixel = op(source_pixel, *target_pixel);
        }
    }
}