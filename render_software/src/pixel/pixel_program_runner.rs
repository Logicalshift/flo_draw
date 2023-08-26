use super::pixel_program_cache::*;

use std::marker::{PhantomData};
use std::ops::{Range};

///
/// Trait implemented by types that can run pixel programs (identified by their data ID)
///
/// `PixelProgramDataCache` is the one provided by this library, but this trait can be re-implemented to customise how a scene is rendered.
///
pub trait PixelProgramRunner {
    /// The type of pixel that this program runner will write
    type TPixel;

    ///
    /// Runs a program with the data found at the `program_data` identifier, to render the pixels in `x_range` to `target`. The pixels will
    /// eventually be rendered at the specified y position in the frame.
    ///
    fn run_program(&self, program_data: PixelProgramDataId, target: &mut [Self::TPixel], x_range: Range<i32>, y_pos: f64);
}

///
/// A pixel program runner that is implemented as a basic function
///
pub struct BasicPixelProgramRunner<TFn, TPixel> 
where
    TFn: Fn(PixelProgramDataId, &mut [TPixel], Range<i32>, f64),
{
    pixel_fn:   TFn,
    pixel:      PhantomData<TPixel>,
}

impl<TFn, TPixel> From<TFn> for BasicPixelProgramRunner<TFn, TPixel>
where
    TFn: Fn(PixelProgramDataId, &mut [TPixel], Range<i32>, f64),
{
    fn from(func: TFn) -> Self {
        BasicPixelProgramRunner { 
            pixel_fn: func, 
            pixel: PhantomData
        }
    }
}

impl<TFn, TPixel> PixelProgramRunner for BasicPixelProgramRunner<TFn, TPixel>
where
    TFn: Fn(PixelProgramDataId, &mut [TPixel], Range<i32>, f64),
{
    type TPixel = TPixel;

    fn run_program(&self, program_data: PixelProgramDataId, target: &mut [Self::TPixel], x_range: Range<i32>, y_pos: f64) {
        (self.pixel_fn)(program_data, target, x_range, y_pos)
    }
}
