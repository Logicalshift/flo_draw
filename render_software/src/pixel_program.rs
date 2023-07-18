use std::ops::{Range};

///
/// A pixel program descibes how to draw pixels along a scan line
///
pub trait PixelProgram {
    /// Data associated with a particular instance of this program
    type ProgramData;

    /// Data for the individual scanlines for this program
    type ScanlineData;

    ///
    /// Draws a series of pixels to a frame buffer
    ///
    /// The target points to the start of the range of values to be written. `x_range` provides the range of X values to 
    ///
    fn draw_pixels(&self, target: &mut [[f32; 4]], x_range: Range<i32>, ypos: i32, program_data: &Self::ProgramData, scanline_data: &Self::ScanlineData);

    ///
    /// Returns the data for a specific scanline
    ///
    /// The x-range here is the range of values intercepted by this program: note that `draw_pixels` may be called with a narrower range if part of
    /// the scan line is not visible, or it is clipped to the edges of the rendering area.
    ///
    fn create_scanline_data(&self, x_range: Range<f32>, ypos: i32, program_data: &Self::ProgramData) -> Self::ScanlineData;
}

impl<TFn> PixelProgram for TFn
where
    TFn: Fn(&mut [[f32; 4]], Range<i32>, i32, &()) -> (),
{
    type ProgramData    = ();
    type ScanlineData   = ();

    #[inline]
    fn draw_pixels(&self, target: &mut [[f32; 4]], x_range: Range<i32>, ypos: i32, program_data: &Self::ProgramData, _scanline_data: &()) {
        (*self)(target, x_range, ypos, program_data)
    }

    #[inline]
    fn create_scanline_data(&self, _x_range: Range<f32>, _ypos: i32, _program_data: &Self::ProgramData) -> () {
        ()
    }
}

///
/// Identifier for a pixel program
///
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PixelProgramId(usize);

///
/// Identifier for the program data for a pixel program
///
/// Every pixel program has a separate set of identifiers for their data
///
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PixelProgramDataId(usize);
