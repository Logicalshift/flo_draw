use super::scanspan::*;
use crate::pixel::*;

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
#[derive(Clone, Debug, PartialEq)]
pub struct ScanSpanStack {
    /// The range of coorindates that these programs should be run over
    pub (crate) x_range:    Range<f64>,

    /// The indexes into the program list that these programs should be run for
    pub (crate) plan:       Range<usize>,

    /// True if these programs do not blend with the background
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
#[derive(Clone, Debug, PartialEq)]
pub struct ScanlinePlan {
    /// Defines which sets of programs are run for which x-range
    spans: Vec<ScanSpanStack>,

    /// The list of programs within this scanline plan
    programs: Vec<PixelProgramPlan>,
}

///
/// Scratch space for the ScanlinePlan::merge() call (used to save on allocations)
///
pub struct ScanlinePlanMergeScratchSpace {
    scratch_space:  Vec<PixelProgramPlan>,
    new_spans:      Vec<ScanSpanStack>,
    new_programs:   Vec<PixelProgramPlan>,
}

impl ScanlinePlanMergeScratchSpace {
    pub fn new() -> Self {
        Self {
            scratch_space:  Vec::with_capacity(64),
            new_spans:      Vec::with_capacity(8),
            new_programs:   Vec::with_capacity(8),
        }
    }
}

impl ScanSpanStack {
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
                plan:       self.plan.clone(),
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
    pub fn programs<'a>(&'a self, scanline_plan: &'a ScanlinePlan) -> impl 'a + Iterator<Item=PixelProgramPlan> {
        scanline_plan.programs[self.plan.clone()].iter().copied()
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
            spans:      Vec::with_capacity(32),
            programs:   Vec::with_capacity(32),
        }
    }
}

impl ScanlinePlan {
    ///
    /// Adds a new set of programs to this plan, which must follow the previous range
    ///
    #[inline]
    pub fn push_next_range(&mut self, x_range: Range<f64>, is_opaque: bool, programs: impl IntoIterator<Item=PixelProgramPlan>) {
        // This is pretty core to performance, so we only verify that the ranges are in the correct order in debug builds
        debug_assert!(self.spans.is_empty() || self.spans.last().unwrap().x_range.end <= x_range.start, "Out of order spans: {:?} < {:?}", x_range, self.spans.last().unwrap().x_range);

        // Add the programs to the programs list
        let start_program_idx = self.programs.len();
        self.programs.extend(programs);
        let end_program_idx = self.programs.len();

        // Add add a description of this range
        self.spans.push(ScanSpanStack { x_range: x_range, plan: start_program_idx..end_program_idx, opaque: is_opaque });
    }

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
    /// Finds any adjacent spans that are running the same program and joins them into one
    ///
    fn combine_adjacent_spans(&mut self) {
        // Search for the first adjacent span...
        for idx in 1..self.spans.len() {
            let last_span = &self.spans[idx-1];
            let this_span = &self.spans[idx];

            if last_span.x_range.end != this_span.x_range.start {
                // Not adjacent
                continue;
            }

            if last_span.plan.len() != this_span.plan.len() || self.programs[last_span.plan.clone()] != self.programs[this_span.plan.clone()] {
                // Different programs
                continue;
            }

            // This is the first span that has a matching adjacent span
            let end = this_span.x_range.end;
            self.spans[idx-1].x_range.end = end;

            // The offset index is where we're moving spans to (one ahead of the 'last' span, and finally the length of the result)
            let mut offset_idx = idx;

            for copy_from_idx in (idx+1)..self.spans.len() {
                let last_span = &self.spans[offset_idx-1];
                let this_span = &self.spans[copy_from_idx];

                if last_span.x_range.end != this_span.x_range.start || (last_span.plan.len() != this_span.plan.len() || self.programs[last_span.plan.clone()] != self.programs[this_span.plan.clone()]) {
                    // Not adjacent, or not matching: copy the definition and update the offset index
                    self.spans[offset_idx] = this_span.clone();
                    offset_idx += 1;
                } else {
                    // this_span is a continuation of last_span: extend it
                    let end = this_span.x_range.end;
                    self.spans[offset_idx-1].x_range.end = end;
                }
            }

            // Truncate the list of spans
            self.spans.truncate(offset_idx);

            // Stop here
            break;
        }
    }

    ///
    /// Merges a scanline plan into this one
    ///
    /// The merged stack is opaque if either stack is opaque. The function is called with the set of pixel programs that are being merged into, the set
    /// from the new program, and whether or not the set in the new program are opaque.
    ///
    pub fn merge(&mut self, merge_with: &ScanlinePlan, merge_stacks: impl Fn(&mut Vec<PixelProgramPlan>, &[PixelProgramPlan], bool)) {
        // TODO: note that we can have issues with performance if we allocate a lot of vecs while rendering: consider re-using the scratch space here.
        // TODO: also consider making a way to do the merge in-place rather than copying the programs out and back in again
        use std::mem;

        let mut scratch = ScanlinePlanMergeScratchSpace::new();

        // Allocate space for the merged spans
        let new_spans           = &mut scratch.new_spans;
        let new_programs        = &mut scratch.new_programs;
        let scratch_space       = &mut scratch.scratch_space;

        new_spans.clear();
        new_programs.clear();
        scratch_space.clear();

        {
            // Iterate on the current and merged spans, and look for overlaps
            let mut our_span_iter       = self.spans.drain(..);
            let mut merge_span_iter     = merge_with.spans.iter().cloned();

            let mut maybe_our_span      = our_span_iter.next();
            let mut maybe_merge_span    = merge_span_iter.next();

            // We iterate both from left to right, and deal with overlaps
            while let (Some(our_span), Some(merge_span)) = (&mut maybe_our_span, &mut maybe_merge_span) {
                if our_span.x_range.end <= merge_span.x_range.start {
                    // our_span is before the merge span
                    new_spans.push(ScanSpanStack {
                        x_range:    our_span.x_range.clone(),
                        plan:       (new_programs.len())..(new_programs.len() + our_span.plan.len()),
                        opaque:     our_span.opaque,
                    });

                    new_programs.extend(self.programs[our_span.plan.clone()].iter().copied());

                    maybe_our_span = our_span_iter.next();
                } else if merge_span.x_range.end <= our_span.x_range.start {
                    // merge_span is before our_span
                    new_spans.push(ScanSpanStack { 
                        x_range:    merge_span.x_range.clone(), 
                        plan:       (new_programs.len())..(new_programs.len() + merge_span.plan.len()), 
                        opaque:     merge_span.opaque });

                    new_programs.extend(merge_with.programs[merge_span.plan.clone()].iter().copied());

                    maybe_merge_span = merge_span_iter.next();
                } else {
                    // The two spans should overlap
                    if merge_span.x_range.start < our_span.x_range.start {
                        // Draw just merge_plan up to our_plan.start
                        new_spans.push(ScanSpanStack {
                            x_range:    merge_span.x_range.start..our_span.x_range.start,
                            plan:       (new_programs.len())..(new_programs.len() + merge_span.plan.len()),
                            opaque:     merge_span.opaque,
                        });

                        new_programs.extend(merge_with.programs[merge_span.plan.clone()].iter().copied());
                    } else if our_span.x_range.start < merge_span.x_range.start {
                        // Draw just our_plan up to our_plan.start
                        new_spans.push(ScanSpanStack {
                            x_range:    our_span.x_range.start..merge_span.x_range.start,
                            plan:       (new_programs.len())..(new_programs.len() + our_span.plan.len()),
                            opaque:     our_span.opaque,
                        });

                        new_programs.extend(self.programs[our_span.plan.clone()].iter().copied());
                    }

                    // Create the merged set of programs. Scratch space is initially empty because it's drained later on.
                    scratch_space.extend(self.programs[our_span.plan.clone()].iter().copied());
                    merge_stacks(scratch_space, &merge_with.programs[merge_span.plan.clone()], merge_span.opaque);

                    // Create the merged plan
                    let start   = our_span.x_range.start.max(merge_span.x_range.start);
                    let end     = our_span.x_range.end.min(merge_span.x_range.end);

                    new_spans.push(ScanSpanStack {
                        x_range:    start..end,
                        plan:       new_programs.len()..(new_programs.len() + scratch_space.len()),
                        opaque:     our_span.opaque || merge_span.opaque,
                    });

                    new_programs.extend(scratch_space.drain(..));

                    // Continue with the remaining part of the plan
                    if end >= our_span.x_range.end {
                        // Entire range was merged
                        maybe_our_span = our_span_iter.next();
                    } else {
                        // Process the remaining part of 'our_span'
                        our_span.x_range.start = end;
                    }

                    if end >= merge_span.x_range.end {
                        // Entire range was merged
                        maybe_merge_span = merge_span_iter.next();
                    } else {
                        // Process the remaining part of 'merge_span'
                        merge_span.x_range.start = end;
                    }
                }
            }

            // Push any remaining spans
            while let Some(our_span) = maybe_our_span {
                let plan = our_span.plan.clone();
                new_spans.push(ScanSpanStack {
                    x_range:    our_span.x_range,
                    opaque:     our_span.opaque,
                    plan:       (new_programs.len())..(new_programs.len() + our_span.plan.len()),
                });
                new_programs.extend(self.programs[plan].iter().copied());

                maybe_our_span = our_span_iter.next();
            } 

            while let Some(merge_span) = maybe_merge_span {
                let plan = merge_span.plan.clone();
                new_spans.push(ScanSpanStack { 
                    x_range:    merge_span.x_range, 
                    opaque:     merge_span.opaque,
                    plan:       (new_programs.len())..(new_programs.len() + merge_span.plan.len()), 
                });
                new_programs.extend(merge_with.programs[plan].iter().copied());

                maybe_merge_span = merge_span_iter.next();
            }
        }

        // Replace the contents of this object with the new spans
        mem::swap(&mut self.spans, new_spans);
        mem::swap(&mut self.programs, new_programs);

        // Combine any adjacent spans that use the same program
        self.combine_adjacent_spans();
    }

    ///
    /// Clears out this plan so the structure can be re-used
    ///
    #[inline]
    pub fn clear(&mut self) {
        self.spans.clear();
        self.programs.clear();
    }

    ///
    /// Returns the programs that should be run to run a particular stack
    ///
    #[inline]
    pub fn programs(&self, stack: &ScanSpanStack) -> &[PixelProgramPlan] {
        &self.programs[stack.plan.clone()]
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
            .flat_map(move |span| {
                let range           = span.x_range();
                let opaque          = span.is_opaque();
                let mut programs    = span.programs(self).filter_map(|program| match program {
                    PixelProgramPlan::Run(program)              => Some(program),
                    PixelProgramPlan::StartBlend                => None,
                    PixelProgramPlan::Merge(_)                  => None,
                    PixelProgramPlan::LinearMerge(_, _)         => None,
                    PixelProgramPlan::SourceOver(_)             => None,
                    PixelProgramPlan::LinearSourceOver(_, _)    => None,
                    PixelProgramPlan::Blend(_, _)               => None,
                    PixelProgramPlan::LinearBlend(_, _, _)      => None,
                });

                // First program is opaque, the rest are transparent
                let first   = if opaque { ScanSpan::opaque(range.clone(), programs.next().unwrap()) } else { ScanSpan::transparent(range.clone(), programs.next().unwrap()) };
                let others  = programs.map(move |program| ScanSpan::transparent(range.clone(), program));

                iter::once(first).chain(others)
            })
    }

    ///
    /// Clips this scanline plan to the specified range, such that x=new_zero_point on the original range is x=0 on the result
    ///
    #[inline]
    pub fn clip(self, source_x_range: Range<f64>, new_zero_point: f64) -> ScanlinePlan {
        let mut new_spans = Vec::with_capacity(self.spans.len());

        // Create the new spans by clipping the old spans
        for span in self.spans.iter() {
            // Skip spans that are entirely of the new range
            if span.x_range.start >= source_x_range.end { break; }
            if span.x_range.end <= source_x_range.start { continue; }

            // Clip the range to the new source range
            let new_span_start  = (span.x_range.start.max(source_x_range.start)) - new_zero_point;
            let new_span_end    = (span.x_range.end.min(source_x_range.end)) - new_zero_point;

            // Clone the old span into the new range
            new_spans.push(ScanSpanStack {
                x_range:    new_span_start..new_span_end,
                plan:       span.plan.clone(),
                opaque:     span.opaque,
            })
        }

        ScanlinePlan {
            spans:      new_spans,
            programs:   self.programs
        }
    }
}
