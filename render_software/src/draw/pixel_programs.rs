use crate::pixel::*;
use crate::pixel_programs::*;

///
/// The standard set of pixel programs for a canvas drawing
///
pub struct CanvasPixelPrograms<TPixel, const N: usize>
where
    TPixel: 'static + Send + Sync + Pixel<N>,
{
    /// The main program cache
    pub (super) program_cache: PixelProgramCache<TPixel>,

    /// The basic solid colour pixel program
    pub (super) solid_color: StoredPixelProgram<SolidColorProgram<TPixel>>,
}

impl<TPixel, const N: usize> Default for CanvasPixelPrograms<TPixel, N> 
where
    TPixel: 'static + Send + Sync + Pixel<N>,
{
    fn default() -> Self {
        let mut cache   = PixelProgramCache::empty();
        let solid_color = cache.add_program(SolidColorProgram::default());

        CanvasPixelPrograms { 
            program_cache:  cache, 
            solid_color:    solid_color,
        }
    }
}

impl<TPixel, const N: usize> CanvasPixelPrograms<TPixel, N> 
where
    TPixel: 'static + Send + Sync + Pixel<N>,
{
    ///
    /// Creates the pixel program data cache to use with the pixel programs
    ///
    #[inline]
    pub fn create_data_cache(&mut self) -> PixelProgramDataCache<TPixel> {
        self.program_cache.create_data_cache()
    }
}
