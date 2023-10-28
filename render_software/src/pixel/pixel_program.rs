use super::alpha_blend_trait::*;
use super::pixel_program_cache::*;

use crate::scanplan::*;

use std::ops::{Range};
use std::marker::{PhantomData};
use std::sync::*;

// A design/performance note:
//
// We pass in the x_transform to the pixel program to provide a way for things like texture brushes to map between pixel coordinates
// and their source coordinates (eg, a canvas drawing has source coordinates going between -1..1 and needs to render to a target
// framebuffer of any dimensions). This means that pixel programs that don't need this transformation - solid colours in particular -
// can avoid the overhead of doing the calculation. However, it also means that if there's a 'stack' of transparent programs that all
// need the transformation, it will be done multiple times per span.
//
// There are two possible approaches: if a scene mostly needs the transform, it would be faster to do it for every span in the scanline
// renderer and pass it in to `draw_pixels`. This will be slower for any scene that doesn't use stacks of programs.
//
// Another approach is to calculate the value lazily and cache it. This is slower for the 'extremes' of the two cases, but might be a
// win overall for either if we don't know what kind of scene is being rendered.

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
    /// The 'x_transform' indicates how the pixel coordinates are translated into source coordinates (the y-position is always given in source
    /// coordinates). If the `x_transform.pixel_range_to_x()` function can be called to generate the source coordinates to render for the
    /// specified x-range.
    ///
    fn draw_pixels(&self, data_cache: &PixelProgramRenderCache<Self::Pixel>, target: &mut [Self::Pixel], x_range: Range<i32>, x_transform: &ScanlineTransform, y_pos: f64, program_data: &Self::ProgramData);
}

///
/// Pixel program that calls a function to fill the pixels, with program data
///
/// This can be used with a pixel program that generates rows of pixels (`PixelProgramFn::from(|target, x_range, ypos, data| { ... })`)
///
pub struct PixelProgramFn<TFn, TPixel, TData>
where 
    TFn: Send + Sync + Fn(&mut [TPixel], Range<i32>, &ScanlineTransform, f64, &TData) -> (),
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
    TFn: Send + Sync + Fn(&mut [TPixel], Range<i32>, &ScanlineTransform, f64, &TData) -> (),
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
    TFn:    Send + Sync + Fn(&mut [TPixel], Range<i32>, &ScanlineTransform, f64, &TData) -> (),
    TData:  Send + Sync,
    TPixel: Send,
{
    type Pixel          = TPixel;
    type ProgramData    = TData;

    #[inline]
    fn draw_pixels(&self, _: &PixelProgramRenderCache<Self::Pixel>, target: &mut [TPixel], x_range: Range<i32>, x_transform: &ScanlineTransform, ypos: f64, program_data: &TData) {
        (self.function)(target, x_range, x_transform, ypos, program_data)
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
    fn draw_pixels(&self, _: &PixelProgramRenderCache<Self::Pixel>, target: &mut [TPixel], x_range: Range<i32>, _x_transform: &ScanlineTransform, ypos: f64, program_data: &TData) {
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
    SourceOver(f32),

    /// Blend the contents of the blend buffer with the current set of pixels, using a linear gradient for the alpha between
    /// the two sets of pixels
    LinearSourceOver(f32, f32),

    /// Blend the contents of the blend buffer with the current set of pixels, using any operation, and release the buffer.
    Blend(AlphaOperation, f32),

    /// Blend the contents of the blend buffer with the current set of pixels, using a linear gradient for the alpha between
    /// the two sets of pixels
    LinearBlend(AlphaOperation, f32, f32),
}
