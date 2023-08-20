use super::scanspan::*;
use crate::pixel::*;

use smallvec::*;

use std::ops::{Range};

// An observation is that we don't have to build up the stacks here, we can just run all the spans from back to front to build up
// the same image, and just split where we want to remove spans underneath opaque spans. This will probably be faster because there
// will be less context switching.
//
// The left-to-right approach here makes it much easier to eliminate rendering behind opaque sections, however. It also operates
// on a more local section of pixels (which can be faster in itself due to how processor caching works), and can be run fully
// in parallel if needed (say, on a GPU as well as on multiple CPUs). It has an advantage with layers too: it's possible to
// process multiple plans together to build up a final result.

///
/// Represents a stack of pixel programs to run on a region of a scanline
///
#[derive(Clone)]
pub struct ScanSpanStack {
    pub (crate) x_range:    Range<f64>,
    pub (crate) first:      PixelProgramPlan,
    pub (crate) others:     Option<SmallVec<[PixelProgramPlan; 4]>>,
    pub (crate) opaque:     bool,
}

///
/// A scanline plan contains the drawing commands needed to draw a single scanline
///
/// Spans in a scanline plan are always stored in order and non-overlapping (that is, the start of the next span is always after the end
/// of the previous span). This means that the full range of the plan can be determined just by checking the first and the last span.
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
            first:      PixelProgramPlan::Run(span.program),
            others:     None,
            opaque:     span.opaque,
        }
    }

    ///
    /// Creates a span stack with the specified set of programs, specified in reverse ordrer
    ///
    #[inline]
    pub fn with_reversed_programs(x_range: Range<f64>, opaque: bool, programs_reversed: &Vec<PixelProgramPlan>) -> ScanSpanStack {
        if programs_reversed.len() == 1 {
            ScanSpanStack {
                x_range:    x_range,
                opaque:     opaque,
                first:      programs_reversed[0],
                others:     None
            }
        } else {
            ScanSpanStack {
                x_range:    x_range,
                first:      programs_reversed[programs_reversed.len()-1],
                others:     Some(programs_reversed.iter().rev().skip(1).copied().collect()),
                opaque:     opaque,
            }
        }
    }

    ///
    /// Adds a new a span to this stack (it will cover the same range as the stack)
    ///
    #[inline]
    pub fn push(&mut self, span: ScanSpan) {
        self.others.get_or_insert_with(|| smallvec![])
            .push(PixelProgramPlan::Run(span.program))
    }

    ///
    /// Splits this stack at an x position (which should be within the range of this span)
    ///
    /// Returns either the right-hand side of the split stack, or an error to indicate that the split point is out of range
    ///
    #[inline]
    pub fn split(&mut self, x_pos: f64) -> Result<ScanSpanStack, ()> {
        if x_pos > self.x_range.start && x_pos < self.x_range.end {
            let end = self.x_range.end;
            self.x_range.end = x_pos;

            Ok(ScanSpanStack {
                x_range:    x_pos..end,
                first:      self.first,
                others:     self.others.clone(),
                opaque:     self.opaque,
            })
        } else {
            Err(())
        }
    }

    ///
    /// The range of pixels covered by this span
    ///
    #[inline]
    pub fn x_range(&self) -> Range<f64> { self.x_range.start..self.x_range.end }

    ///
    /// Returns an iterator for the IDs of the programs that should be run over this range
    ///
    #[inline]
    pub fn programs<'a>(&'a self) -> impl 'a + Iterator<Item=PixelProgramPlan> {
        use std::iter;

        iter::once(self.first)
            .chain(self.others.iter().flatten().copied())
    }

    ///
    /// True if this stack is opaque (will overwrite anything it's drawn on top of), false if it's transparent (will blend)
    /// with anything it's on top of
    ///
    #[inline]
    pub fn is_opaque(&self) -> bool { self.opaque }
}

impl Default for ScanlinePlan {
    fn default() -> Self {
        ScanlinePlan {
            spans: vec![]
        }
    }
}

impl ScanlinePlan {
    ///
    /// Asserts that a list of stacks is in the correct order and non-overlapping, so that we know that the plan is safe to use without
    /// bounds checking
    ///
    pub fn check_spans_ordering(stacks: &Vec<ScanSpanStack>) {
        let mut stack_iter = stacks.iter();

        if let Some(first_stack) = stack_iter.next() {
            let mut last_x = first_stack.x_range.end;

            while let Some(next_stack) = stack_iter.next() {
                assert!(next_stack.x_range.start >= last_x, "Spans are out of order ({} < {})", next_stack.x_range.start, last_x);
                assert!(next_stack.x_range.start != next_stack.x_range.end, "0-length span");

                last_x = next_stack.x_range.end;
            }
        }
    }

    ///
    /// Creates a scanline plan from a set of ScanSpanStacks which are non-overlapping and ordered from left-to-right
    ///
    #[inline]
    pub fn from_ordered_stacks(stacks: Vec<ScanSpanStack>) -> ScanlinePlan {
        Self::check_spans_ordering(&stacks);

        unsafe { Self::from_ordered_stacks_prechecked(stacks) }
    }

    ///
    /// Replaces a scanline plan with a set of ScanSpanStacks that are expected to be in order and non-overlapping
    ///
    /// This is marked as 'unsafe' because we later depend on these stacks to be non-overlapping for safety reasons. Call
    /// `from_ordered_stacks` instead to create a plan with checking.
    ///
    #[inline]
    pub unsafe fn from_ordered_stacks_prechecked(stacks: Vec<ScanSpanStack>) -> ScanlinePlan {
        ScanlinePlan {
            spans: stacks
        }
    }

    ///
    /// Creates a scanline plan from a set of ScanSpanStacks which are non-overlapping and ordered from left-to-right
    ///
    #[inline]
    pub fn fill_from_ordered_stacks(&mut self, stacks: Vec<ScanSpanStack>) {
        Self::check_spans_ordering(&stacks);

        unsafe { self.fill_from_ordered_stacks_prechecked(stacks) }
    }

    ///
    /// Replaces a scanline plan with a set of ScanSpanStacks that are expected to be in order and non-overlapping
    ///
    /// This is marked as 'unsafe' because we later depend on these stacks to be non-overlapping for safety reasons. Call
    /// `from_ordered_stacks` instead to create a plan with checking.
    ///
    #[inline]
    pub unsafe fn fill_from_ordered_stacks_prechecked(&mut self, stacks: Vec<ScanSpanStack>) {
        self.spans = stacks;
    }

    ///
    /// Adds a new span to this plan
    ///
    pub fn add_span(&mut self, span: ScanSpan) {
        // Binary search for where this span begins
        let x_pos   = span.x_range.start;
        let mut min = 0;
        let max     = self.spans.len();

        /* -- TODO, test is this worth it? (as we just insert into the vec later on)
        while max > min+4 {
            // Calculate mid-point
            let mid     = (min + max) >> 1;
            let mid_pos = self.spans[mid].x_range.end;

            if mid_pos <= x_pos {
                min = mid + 1;
            } else {
                max = mid;
            }
        }
        */

        // Linear search for small ranges
        while min < max {
            let min_pos = self.spans[min].x_range.end;
            if min_pos > x_pos {
                break;
            }

            min += 1;
        }

        // The position that's >= the start of the span
        let mut pos = min;

        // Try to split the span at pos (the current span might start after the start of the position)
        if pos < self.spans.len() {
            match self.spans[pos].split(span.x_range.start) {
                Ok(rhs) => {
                    // Add the RHS into the spans to be merged by the remainder of the algorithm
                    self.spans.insert(pos+1, rhs);
                    pos += 1;
                }

                Err(()) => { }
            }
        }

        // Add the span to the stacks by repeatedly splitting it
        if span.opaque {
            // Span is opaque: replace existing stacks with it, combine/delete them rather than split them
            let span = span;

            if pos >= self.spans.len() {
                // This span is after the end of the current stack
                self.spans.push(ScanSpanStack::with_first_span(span));
            } else if span.x_range.end < self.spans[pos].x_range.start {
                // The span is in between any existing span
                self.spans.insert(pos, ScanSpanStack::with_first_span(span));
            } else if span.x_range.end == self.spans[pos].x_range.end {
                // The span exactly replaces the current span
                self.spans[pos] = ScanSpanStack::with_first_span(span);
            } else if span.x_range.end < self.spans[pos].x_range.end {
                // The span overlaps the start of the current span (can't overlap the middle due to the split operation above)
                self.spans[pos].x_range.start = span.x_range.end;
                self.spans.insert(pos, ScanSpanStack::with_first_span(span));
            } else {
                // The span overlaps the existing span, and maybe the spans in front of it
                let x_range = span.x_range.clone();
                self.spans[pos] = ScanSpanStack::with_first_span(span);
                pos += 1;

                loop {
                    if pos >= self.spans.len() { break; }
                    if self.spans[pos].x_range.start >= x_range.end { break; }

                    if self.spans[pos].x_range.end > x_range.end {
                        self.spans[pos].x_range.start = x_range.end;
                        break;
                    }

                    self.spans.remove(pos);
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
    /// Clears out this plan so the structure can be re-used
    ///
    #[inline]
    pub fn clear(&mut self) {
        self.spans.clear();
    }

    ///
    /// Returns the spans in this plan
    ///
    #[inline]
    pub fn spans(&self) -> &[ScanSpanStack] {
        &self.spans
    }

    ///
    /// Generates scan spans in rendering order for this scanline
    ///
    /// The lowest span in a stack is always returned as opaque even if it was originally created as transparent using this function
    ///
    #[inline]
    pub fn iter_as_stacks<'a>(&'a self) -> impl 'a + Iterator<Item=&'a ScanSpanStack> {
        self.spans.iter()
    }

    ///
    /// Generates scan spans in rendering order for this scanline
    ///
    /// The lowest span in a stack is always returned as opaque even if it was originally created as transparent using this function. Blending is ignored
    /// in these results.
    ///
    #[inline]
    pub fn iter_as_spans<'a>(&'a self) -> impl 'a + Iterator<Item=ScanSpan> {
        use std::iter;

        self.iter_as_stacks()
            .flat_map(|span| {
                let range           = span.x_range();
                let opaque          = span.is_opaque();
                let mut programs    = span.programs().filter_map(|program| match program {
                    PixelProgramPlan::Run(program)          => Some(program),
                    PixelProgramPlan::StartBlend            => None,
                    PixelProgramPlan::Blend(_)              => None,
                    PixelProgramPlan::LinearBlend(_, _)     => None,
                });

                // First program is opaque, the rest are transparent
                let first   = if opaque { ScanSpan::opaque(range.clone(), programs.next().unwrap()) } else { ScanSpan::transparent(range.clone(), programs.next().unwrap()) };
                let others  = programs.map(move |program| ScanSpan::transparent(range.clone(), program));

                iter::once(first).chain(others)
            })
    }
}
