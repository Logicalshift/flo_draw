use super::scanspan::*;
use crate::pixel_program_cache::*;

use std::ops::{Range};

// An observation is that we don't have to build up the stacks here, we can just run all the spans from back to front to build up
// the same image, and just split where we want to remove spans underneath opaque spans. This will probably be faster because there
// will be less context switching.
//
// The left-to-right approach here makes it much easier to eliminate rendering behind opaque sections, however. It also operates
// on a more local section of pixels (which can be faster in itself due to how processor caching works), and can be run fully
// in parallel if needed (say, on a GPU as well as on multiple CPUs). It has an advantage with layers too: it's possible to
// process multiple plans together to build up a final result.

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
    ///
    /// Creates a new stack containing a single span
    ///
    #[inline]
    pub fn with_first_span(span: ScanSpan) -> ScanSpanStack {
        ScanSpanStack { 
            x_range:    span.x_range,
            first:      span.program,
            others:     None
        }
    }

    ///
    /// Adds a new a span to this stack (it will cover the same range as the stack)
    ///
    #[inline]
    pub fn push(&mut self, span: ScanSpan) {
        self.others.get_or_insert_with(|| vec![])
            .push(span.program)
    }

    ///
    /// Splits this stack at an x position (which should be within the range of this span)
    ///
    /// Returns either the right-hand side of the split stack, or an error to indicate that the split point is out of range
    ///
    #[inline]
    pub fn split(&mut self, x_pos: i32) -> Result<ScanSpanStack, ()> {
        if x_pos >= self.x_range.start && x_pos < self.x_range.end {
            let end = self.x_range.end;
            self.x_range.end = x_pos;

            Ok(ScanSpanStack {
                x_range:    x_pos..end,
                first:      self.first,
                others:     self.others.clone(),
            })
        } else {
            Err(())
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

    ///
    /// Adds a new span to this plan
    ///
    pub fn add_span(&mut self, span: ScanSpan) {
        use std::mem;

        // Binary search for where this span begins
        let x_pos   = span.x_range.start;
        let mut min = 0;
        let max     = self.spans.len();

        /* -- TODO, test is this worth it? (as we just insert into the vec later on)
        while max > min+4 {
            // Calculate mid-point
            let mid     = (min + max) >> 1;
            let mid_pos = self.spans[mid].x_range.start;

            if mid_pos == x_pos {
                min = mid;
                max = min;
                break;
            } else if mid_pos < x_pos {
                min = mid + 1;
            } else {
                max = mid;
            }
        }
        */

        // Linear search for small ranges
        while min < max {
            let min_pos = self.spans[min].x_range.end;
            if min_pos >= x_pos {
                break;
            }

            min += 1;
        }

        // The position that's >= the start of the span
        let mut pos = min;

        // Add the span to the stacks by repeatedly splitting it
        if span.opaque {
            // Span is opaque: replace existing stacks with it
            let mut span = span;

            loop {
                if pos >= self.spans.len() {
                    // This span is after the end of the current stack
                    self.spans.push(ScanSpanStack::with_first_span(span));
                    break;
                }

                if self.spans[pos].x_range.start > span.x_range.start {
                    // Scanline is before this range: split it at the start of the range if possible
                    match span.split(self.spans[pos].x_range.start) {
                        Ok((lhs, rhs)) => {
                            // LHS needs to be added as a new span
                            self.spans.insert(pos, ScanSpanStack::with_first_span(lhs));

                            // Remaining span is the RHS
                            span = rhs;

                            // Move the position back to the original span (we now know that it overlaps this range)
                            pos += 1;
                        }

                        Err(span) => {
                            // Span just fits before the current position
                            self.spans.insert(pos, ScanSpanStack::with_first_span(span));
                            break;
                        }
                    }
                }

                // Scanline overlaps this range: split it at the end of the current range if possible
                match span.split(self.spans[pos].x_range.end) {
                    Ok((lhs, rhs)) => {
                        // Remaining part of the new span on the rhs
                        self.spans[pos] = ScanSpanStack::with_first_span(lhs);
                        span            = rhs;

                        // New position is after the current span
                        pos += 1;
                    }

                    Err(span) => {
                        // Swap out the exisitng stack
                        let end = span.x_range.end;

                        let mut remaining = ScanSpanStack::with_first_span(span);
                        mem::swap(&mut self.spans[pos], &mut remaining);

                        // Add the 'remaining' stack back in if the existing span doesn't fully overlap it
                        if remaining.x_range.end > end {
                            remaining.x_range.start = end;
                            self.spans.insert(pos+1, remaining);
                        }

                        break;
                    }
                }
            }
        } else {
            // Span is transparent: add to existing stacks
            let mut span = span;

            loop {
                if pos >= self.spans.len() {
                    // This span is after the end of the current stack
                    self.spans.push(ScanSpanStack::with_first_span(span));
                    break;
                }

                if self.spans[pos].x_range.start > span.x_range.start {
                    // Scanline is before this range: split it at the start of the range if possible
                    match span.split(self.spans[pos].x_range.start) {
                        Ok((lhs, rhs)) => {
                            // LHS needs to be added as a new span
                            self.spans.insert(pos, ScanSpanStack::with_first_span(lhs));

                            // Remaining span is the RHS
                            span = rhs;

                            // Move the position back to the original span (we now know that it overlaps this range)
                            pos += 1;
                        }

                        Err(span) => {
                            // Span just fits before the current position
                            self.spans.insert(pos, ScanSpanStack::with_first_span(span));
                            break;
                        }
                    }
                }

                // Scanline overlaps this range: split it at the end of the current range if possible
                match span.split(self.spans[pos].x_range.end) {
                    Ok((lhs, rhs)) => {
                        // Remaining part of the new span on the rhs
                        self.spans[pos].push(lhs);
                        span = rhs;

                        // New position is after the current span
                        pos += 1;
                    }

                    Err(span) => {
                        // Span either entirely overlaps the range, or partially overlaps it at the start
                        match self.spans[pos].split(span.x_range.end) {
                            Ok(rhs) => {
                                // Span overlaps the start of the range
                                self.spans[pos].push(span);

                                // The RHS is the parts of the span
                                self.spans.insert(pos+1, rhs);
                            }

                            Err(()) => {
                                // Add the current span to the
                                self.spans[pos].push(span);
                            }
                        }

                        // Span is entirely consumed
                        break;
                    }
                }
            }
        }
    }

    ///
    /// Generates scan spans in rendering order for this scanline
    ///
    /// The lowest span in a stack is always returned as opaque even if it was originally created as transparent using this function
    ///
    pub fn iter_as_spans<'a>(&'a self) -> impl 'a + Iterator<Item=ScanSpan> {
        // TODO: should the lowest span always be returned as opaque? Layers might be more efficient to implement if we can know if their lowest span is opaque or transparent
        use std::iter;

        self.spans.iter()
            .flat_map(|span| {
                let range = span.x_range.clone();

                iter::once(ScanSpan::opaque(range.clone(), span.first))
                    .chain(span.others.iter().flatten().copied().map(move |stack_program| {
                        ScanSpan::transparent(range.clone(), stack_program)
                    }))
            })
    }
}