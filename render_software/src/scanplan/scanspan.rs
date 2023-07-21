use crate::pixel_program_cache::*;

use std::ops::{Range};

///
/// A ScanSpan indicates which program(s) to apply to a range along a scanline 
///
#[derive(Clone)]
pub struct ScanSpan {
    /// The pixels to draw on the scanline
    x_range: Range<i32>,

    /// The ID of the scanline data for the program to run over this range
    program: PixelScanlineDataId
}

impl ScanSpan {
    ///
    /// Creates a scanspan that will run a single program
    ///
    #[inline]
    pub fn new(range: Range<i32>, program: PixelScanlineDataId) -> ScanSpan {
        ScanSpan {
            x_range: range,
            program: program
        }
    }

    ///
    /// Splits this span at the specified position
    ///
    /// Returns the same span if the split would result in a 0-length span
    ///
    #[inline]
    pub fn split(self, pos: i32) -> Result<(ScanSpan, ScanSpan), ScanSpan> {
        if pos >= self.x_range.start && pos < self.x_range.end {
            Ok((
                ScanSpan {
                    x_range: (self.x_range.start)..pos,
                    program: self.program,
                },
                ScanSpan {
                    x_range: pos..(self.x_range.end),
                    program: self.program,
                }
            ))
        } else {
            Err(self)
        }
    }
}
