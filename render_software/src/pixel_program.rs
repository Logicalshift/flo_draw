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
    fn draw_pixels(target: &[[f32; 4]], x_range: Range<i32>, ypos: i32, program_data: &Self::ProgramData, scanline_data: &Self::ScanlineData);
}
