use crate::pixel_program_cache::*;

use std::ops::{Range};

///
/// A ScanSpan indicates which program(s) to apply to a range along a scanline 
///
#[derive(Clone)]
pub struct ScanSpan {
    /// The pixels to draw on the scanline
    x_range: Range<i32>,

    /// The data ID for the program to run over this scanline
    program: PixelScanlineDataId,
}
