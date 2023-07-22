use super::scanspan::*;
use crate::pixel_program_cache::*;

use std::ops::{Range};

// An observation is that we don't have to build up the stacks here, we can just run all the spans from back to front to build up
// the same image, and just split where we want to remove spans underneath opaque spans. This will probably be faster because there
// will be less context switching.
//
// The left-to-right approach here makes it much easier to eliminate rendering behind opaque sections, however. It also operates
// on a more local section of pixels (which can be faster in itself due to how processor caching works), and can be run fully
// in parallel if needed (say, on a GPU as well as on multiple CPUs)

#[derive(Clone)]
struct ScanSpanStack {
    x_range:    Range<i32>,
    first:      PixelScanlineDataId,
    others:     Option<Vec<PixelScanlineDataId>>,
}

///
/// A scanline plan contains the drawing commands needed to draw a single scanline
///
/// The scanline is divided up into 'stacks' of `ScanSpan`s, moving from left to right (so scanlines are always drawn from left-to-right).
/// This class builds up the plan to draw the scanline by adding new `ScanSpan`s and merging and splitting them to make the stacks.
///
#[derive(Clone)]
pub struct ScanlinePlan {
    spans: Vec<ScanSpanStack>,
}

impl ScanSpanStack {
    /// Creates a new stack containing a single span
    #[inline]
    pub fn with_first_span(span: ScanSpan) -> ScanSpanStack {
        ScanSpanStack { 
            x_range:    span.x_range,
            first:      span.program,
            others:     None
        }
    }

    /// Adds a new a span to this stack (it will cover the same range as the stack)
    #[inline]
    pub fn push(&mut self, span: ScanSpan) {
        self.others.get_or_insert_with(|| vec![])
            .push(span.program)
    }

    /// Splits this stack at an x position
    #[inline]
    pub fn split(self, x_pos: i32) -> Result<(ScanSpanStack, ScanSpanStack), ScanSpanStack> {
        if x_pos >= self.x_range.start && x_pos < self.x_range.end {
            Ok((
                ScanSpanStack {
                    x_range:    (self.x_range.start)..x_pos,
                    first:      self.first,
                    others:     self.others.clone(),
                },
                ScanSpanStack {
                    x_range:    x_pos..(self.x_range.end),
                    first:      self.first,
                    others:     self.others.clone(),
                }
            ))
        } else {
            Err(self)
        }
    }
}

impl ScanlinePlan {
    ///
    /// Creates a new scanline plan (with no spans)
    ///
    pub fn new() -> ScanlinePlan {
        ScanlinePlan {
            spans: vec![]
        }
    }
}