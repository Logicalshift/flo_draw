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
    fn draw_pixels(&self, target: &[[f32; 4]], x_range: Range<i32>, ypos: i32, program_data: &Self::ProgramData, scanline_data: &Self::ScanlineData);

    ///
    /// Returns the data for a specific scanline
    ///
    /// The x-range here is the range of values intercepted by this program: note that `draw_pixels` may be called with a narrower range if part of
    /// the scan line is not visible, or it is clipped to the edges of the rendering area.
    ///
    fn create_scanline_data(&self, target: &[[f32; 4]], x_range: Range<f32>, ypos: i32, program_data: &Self::ProgramData) -> Self::ScanlineData;
}

///
/// Identifier for a pixel program
///
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PixelProgramId(usize);
