use super::pixel_program_cache::*;

use std::ops::{Range};
use std::marker::{PhantomData};
use std::sync::*;

///
/// A pixel program descibes how to draw pixels along a scan line
///
pub trait PixelProgram : Send + Sync {
    /// The type representing a pixel within this program
    type Pixel : Send;

    /// Data associated with a particular instance of this program
    type ProgramData : Send + Sync;

    ///
    /// Draws a series of pixels to a frame buffer
    ///
    /// The target points to the start of the range of values to be written. `x_range` provides the range of X values to fill with pixels.
    ///
    fn draw_pixels(&self, data_cache: &PixelProgramDataCache<Self::Pixel>, target: &mut [Self::Pixel], x_range: Range<i32>, y_pos: f64, program_data: &Self::ProgramData);
}

///
/// Pixel program that calls a function to fill the pixels, with program data
///
/// This can be used with a pixel program that generates rows of pixels (`PixelProgramFn::from(|target, x_range, ypos, data| { ... })`)
///
pub struct PixelProgramFn<TFn, TPixel, TData>
where 
    TFn: Send + Sync + Fn(&mut [TPixel], Range<i32>, f64, &TData) -> (),
{
    /// The function to call to fill in the pixels
    function: TFn,

    /// Placeholder for the TData type (Rust doesn't see a function parameter as a constraint)
    phantom_data: PhantomData<Mutex<(TData, TPixel)>>,
}

///
/// Pixel program that calls a function to fill the pixels, with program data
///
/// This can be used with a pixel program that generates individual pixels (`PerPixelProgramFn::from(|x, y, data| { [r, g, b, a] })`)
///
pub struct PerPixelProgramFn<TFn, TPixel, TData>
where 
    TFn: Send + Sync + Fn(i32, f64, &TData) -> TPixel,
{
    /// The function to call to fill in the pixels
    function: TFn,

    /// Placeholder for the TData type (Rust doesn't see a function parameter as a constraint)
    phantom_data: PhantomData<Mutex<(TData, TPixel)>>,
}

impl<TFn, TPixel, TData> From<TFn> for PixelProgramFn<TFn, TPixel, TData> 
where 
    TFn: Send + Sync + Fn(&mut [TPixel], Range<i32>, f64, &TData) -> (),
{
    fn from(function: TFn) -> Self {
        PixelProgramFn {
            function:       function,
            phantom_data:   PhantomData,
        }
    }
}

impl<TFn, TPixel, TData> PixelProgram for PixelProgramFn<TFn, TPixel, TData> 
where 
    TFn:    Send + Sync + Fn(&mut [TPixel], Range<i32>, f64, &TData) -> (),
    TData:  Send + Sync,
    TPixel: Send,
{
    type Pixel          = TPixel;
    type ProgramData    = TData;

    #[inline]
    fn draw_pixels(&self, _: &PixelProgramDataCache<Self::Pixel>, target: &mut [TPixel], x_range: Range<i32>, ypos: f64, program_data: &TData) {
        (self.function)(target, x_range, ypos, program_data)
    }
}

impl<TFn, TPixel, TData> From<TFn> for PerPixelProgramFn<TFn, TPixel, TData> 
where 
    TFn: Send + Sync + Fn(i32, f64, &TData) -> TPixel,
{
    fn from(function: TFn) -> Self {
        PerPixelProgramFn {
            function:       function,
            phantom_data:   PhantomData,
        }
    }
}

impl<TFn, TPixel, TData> PixelProgram for PerPixelProgramFn<TFn, TPixel, TData> 
where 
    TFn:    Send + Sync + Fn(i32, f64, &TData) -> TPixel,
    TData:  Send + Sync,
    TPixel: Send,
{
    type Pixel          = TPixel;
    type ProgramData    = TData;

    #[inline]
    fn draw_pixels(&self, _: &PixelProgramDataCache<Self::Pixel>, target: &mut [TPixel], x_range: Range<i32>, ypos: f64, program_data: &TData) {
        let mut pos = 0;
        for x in x_range {
            target[pos] = (self.function)(x, ypos, program_data);
            pos += 1;
        }
    }
}

///
/// Identifier for a pixel program
///
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct PixelProgramId(pub usize);

///
/// Evaluation plan for a pixel program
///
/// Pixel programs can be run directly on the contents that are underneath them, or blended with the contents. While
/// blending could also be done by creating blending programs and data segments, it's easier if the scan conversion
/// algorithms can specify a partial blend of a program stack.
///
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug)]
pub enum PixelProgramPlan {
    /// Run the pixel program on the current scanline data, with no further processing
    Run(PixelProgramDataId),

    /// Run the following programs into a 'blend buffer', which is committed to the results by one of the blending operations
    ///
    /// A blend buffer is created every time this operation is performed, and is initialised with the current set of pixels.
    /// The buffer is destroyed when one of the blend operations is performed. Blend buffers form a stack, so it's possible
    /// to nest blending operations.
    StartBlend,

    /// Blend the contents of the blend buffer with the current set of pixels, using the source-over operation, and release
    /// the buffer.
    Blend(f32),

    /// Blend the contents of the blend buffer with the current set of pixels, using a linear gradient for the alpha between
    /// the two sets of pixels
    LinearBlend(f32, f32),
}
