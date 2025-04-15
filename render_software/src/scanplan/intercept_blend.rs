use super::alpha_coverage::*;
use crate::edgeplan::*;
use crate::pixel::*;

use std::ops::{Range};

///
/// Ways that a scanline fragment can be blended with the background
///
#[derive(Clone, Debug)]
pub enum InterceptBlend {
    /// Only the shape's blending should be used
    Solid,

    /// This should be alpha-blended with a linear fade, where the alpha value `a = a*x + b`
    LinearFade { a: f64, b: f64 },

    /// Specifies a linear fade that changes to a different blend after a certain point
    LinearFadeWithLimit { a: f64, b: f64, limit: f64, next: Box<InterceptBlend> },
}

///
/// Computes the range that a line `ax + b` where it crosses y=0 and y=1
///
fn range_for_line(a: f64, b: f64) -> Range<f64> {
    if a == b {
        0.0..0.0
    } else {
        let zero_pos    = (0.0-b)/a;
        let one_pos     = (1.0-b)/a;

        (zero_pos.min(one_pos))..(zero_pos.max(one_pos))
    }
}

///
/// 'Applies' one slope to another
///
/// Fades affect the region from which they go from alpha=0 to alpha=1
///
/// Slopes that move in the same direction will be added together, stopping when they are saturated at alpha=1 or alpha=0.
/// Slopes that move in opposite directions will either fade in then out or the other way around. 
///
fn apply(a1: f64, b1: f64, a2: f64, b2: f64) -> InterceptBlend {
    // The slope of the new fade is calculated by adding the coefficients
    let a3 = a1 + a2;
    let b3 = b1 + b2;

    // Compute the ranges for the various distances
    let range1 = range_for_line(a1, b1);
    let range2 = range_for_line(a2, b2);

    if range1.end == range2.end {
        InterceptBlend::LinearFade { a: a3, b: b3 }
    } else if range1.end < range2.end {
        // Range2 carries on for longer than range1
        if range2.start > range1.end {
            // Ranges don't overlap
            InterceptBlend::LinearFadeWithLimit {
                a: a1, b: b1,
                limit: range2.start,
                next: Box::new(InterceptBlend::LinearFade { 
                    a: a2, b: b2, 
                })
            }
        } else if range2.start > range1.start {
            // Range2 starts after range1
            InterceptBlend::LinearFadeWithLimit {
                a: a1, b: b1,
                limit: range2.start,
                next: Box::new(InterceptBlend::LinearFadeWithLimit { 
                    a: a3, b: b3, 
                    limit: range1.end, 
                    next: Box::new(InterceptBlend::LinearFade {
                        a: a2, b: b2
                    })
                })
            }
        } else {
            // Range1 starts after range2
            InterceptBlend::LinearFadeWithLimit {
                a: a3, b: b3,
                limit: range1.end,
                next: Box::new(InterceptBlend::LinearFade {
                    a: a2, b: b2
                })
            }
        }
    } else {
        // Range1 carries on for longer than range2
        if range1.start > range2.end {
            // Ranges don't overlap
            InterceptBlend::LinearFadeWithLimit {
                a: a2, b: b2,
                limit: range1.start,
                next: Box::new(InterceptBlend::LinearFade { 
                    a: a1, b: b1, 
                })
            }
        } else if range1.start > range2.start {
            // Range1 starts after range2
            InterceptBlend::LinearFadeWithLimit {
                a: a2, b: b2,
                limit: range1.start,
                next: Box::new(InterceptBlend::LinearFadeWithLimit { 
                    a: a3, b: b3, 
                    limit: range1.end, 
                    next: Box::new(InterceptBlend::LinearFade {
                        a: a1, b: b1
                    })
                })
            }
        } else {
            // Range2 starts after range1
            InterceptBlend::LinearFadeWithLimit {
                a: a3, b: b3,
                limit: range2.end,
                next: Box::new(InterceptBlend::LinearFade {
                    a: a1, b: b1
                })
            }
        }
    }
}

impl InterceptBlend {
    ///
    /// Creates a linear fade that has an alpha of 0 at zero_x and 1 at one_x. The is_inside value determines whether or not the
    /// left-hand side of the fade is inside or outside
    ///
    #[inline]
    pub fn linear_fade(zero_x: f64, one_x: f64, is_inside: bool) -> InterceptBlend {
        if zero_x == one_x {
            // For vertical lines, treat them as very slightly slanted (which saves us having to special case them)
            let a = if is_inside { -1.0/1e-6 } else { 1.0/1e-6 };
            let b = 1.0-a*one_x;

            InterceptBlend::LinearFade { a, b }
        } else {
            let a = 1.0/(one_x-zero_x);
            let b = 1.0-a*one_x;

            InterceptBlend::LinearFade { a, b }
        }
    }

    ///
    /// If this is a fade blend, multiply the ratios by the specified number
    ///
    pub fn multiply_fade(&self, factor: f64) -> InterceptBlend {
        match self {
            InterceptBlend::Solid                                       => InterceptBlend::Solid,
            InterceptBlend::LinearFade { a, b }                         => InterceptBlend::LinearFade { a: a*factor, b: b*factor },
            InterceptBlend::LinearFadeWithLimit { a, b, limit, next }   => InterceptBlend::LinearFadeWithLimit { a: a*factor, b: b*factor, limit: *limit, next: Box::new(next.multiply_fade(factor)) },
        }
    }

    ///
    /// Nests another fade blend inside this one: this creates a new blend that's as if the new blend is drawn on top of this one, adding the values together.
    ///
    /// We assume that only points after the start of both blends will need to be generated.
    ///
    pub fn nest(&self, blend: InterceptBlend) -> InterceptBlend {
        // Recursively merge the split values and the following values
        fn merge(blend: InterceptBlend, max_limit: f64, after_max_limit: InterceptBlend) -> InterceptBlend {
            // Ensure that everything 'after_max_limit' is actually after that point
            let mut after_max_limit = after_max_limit;
            loop {
                match &after_max_limit {
                    InterceptBlend::LinearFadeWithLimit { limit, next, .. } => {
                        if *limit <= max_limit {
                            // TODO: can just keep using 'next' here but it's a pain to reconstruct everything
                            after_max_limit = (**next).clone();
                        } else {
                            break;
                        }
                    }

                    _ => { break; }
                }
            }

            // Copy anything from the orginal blend that's before the max limit
            match blend {
                InterceptBlend::LinearFade { a, b } => {
                    InterceptBlend::LinearFadeWithLimit { a: a, b: b, limit: max_limit, next: Box::new(after_max_limit) }
                }

                InterceptBlend::LinearFadeWithLimit { a, b, limit, next } => {
                    if limit > max_limit {
                        InterceptBlend::LinearFadeWithLimit { a: a, b: b, limit: max_limit, next: Box::new(after_max_limit) }
                    } else {
                        InterceptBlend::LinearFadeWithLimit { a: a, b: b, limit: limit, next: Box::new(merge(*next, max_limit, after_max_limit))}   
                    }
                }

                _ => todo!("Should not be reachable"), // ... because this should all be linear fades at this point
            }
        }

        match self {
            InterceptBlend::Solid                                               => InterceptBlend::Solid,
            InterceptBlend::LinearFade { a, b }                                 => {
                match &blend {
                    InterceptBlend::Solid                                       => InterceptBlend::Solid,
                    InterceptBlend::LinearFade { a: a2, b: b2 }                 => apply(*a, *b, *a2, *b2),
                    InterceptBlend::LinearFadeWithLimit { .. }   => {
                        // TODO: is this inefficient? I think the result is the same, but we're doing a nested call and re-checking the blends
                        blend.nest(self.clone())
                    }
                }
            },

            InterceptBlend::LinearFadeWithLimit { a, b, limit, next } => {
                match blend {
                    InterceptBlend::Solid   => InterceptBlend::Solid,
                    InterceptBlend::LinearFade { a: a2, b: b2 } => {
                        // Don't want to merge any more after the end of range2
                        let range2 = range_for_line(a2, b2);

                        // Split this section of the line
                        let split = apply(*a, *b, a2, b2);

                        match split {
                            InterceptBlend::Solid                       => InterceptBlend::Solid,
                            InterceptBlend::LinearFade { a: a3, b: b3 } => {
                                if *limit < range2.end {
                                    // The new blend continues beyond the limit, so apply it to the remainder
                                    InterceptBlend::LinearFadeWithLimit { a: a3, b: b3, limit: *limit, next: Box::new(next.nest(blend)) }
                                } else {
                                    // New blend ends before the limit, so the remainder stays the same
                                    InterceptBlend::LinearFadeWithLimit { a: a3, b: b3, limit: *limit, next: next.clone() }
                                }
                            },

                            InterceptBlend::LinearFadeWithLimit { a: a3, b: b3, limit: limit3, next: next3 } => {
                                // We've split up the initial linear fade into multiple sections. What we do here is take everything from the split
                                // fade up to the original limit, then do another nest with the 'next' section.
                                let next = if *limit < range2.end {
                                    // Following the section we've split up still overlaps the new fade
                                    next.nest(blend)
                                } else {
                                    // The fade does not continue after the limit, so we can keep the next parts the same
                                    (**next).clone()
                                };

                                merge(Self::LinearFadeWithLimit { a: a3, b: b3, limit: limit3, next: next3 }, *limit, next)
                            }
                        }
                    },

                    InterceptBlend::LinearFadeWithLimit { a: a2, b: b2, limit: limit2, next: next2 }  => {
                        // We have a linear section up to the lower of the two limits: this forms the LHS of the new section
                        let new_limit   = limit.min(limit2);

                        let new_lhs1    = InterceptBlend::LinearFade { a: *a, b: *b };
                        let new_lhs2    = InterceptBlend::LinearFade { a: a2, b: b2 };
                        let new_lhs     = new_lhs1.nest(new_lhs2);

                        // If the limits are different, one RHS is split in two
                        let new_rhs1 = if *limit > limit2 {
                            // Split 'self' up at 'limit2'
                            self.clone()
                        } else {
                            // 'self' is entirely consumed by the LHS
                            (**next).clone()
                        };
                        let new_rhs2 = if limit2 > *limit {
                            // Split 'blend' up at 'limit'
                            InterceptBlend::LinearFadeWithLimit { a: a2, b: b2, limit: limit2, next: next2 }
                        } else {
                            // 'blend' is entirely consumed by the LHS
                            *next2
                        };

                        // Combine the right-hand sides recursively
                        let new_rhs = new_rhs1.nest(new_rhs2);

                        // Merge the left and right-hand sides to generate the final result
                        merge(new_lhs, new_limit, new_rhs)
                    }
                }
            },
        }
    }

    ///
    /// Returns the range where this fade moves between 0 and 1
    ///
    pub fn range(&self) -> Range<f64> {
        match self {
            InterceptBlend::Solid => 0.0..0.0,

            InterceptBlend::LinearFade { a, b } => {
                if a == b {
                    0.0..0.0
                } else {
                    let zero_pos    = (0.0-b)/a;
                    let one_pos     = (1.0-b)/a;

                    (zero_pos.min(one_pos))..(zero_pos.max(one_pos))
                }
            }

            InterceptBlend::LinearFadeWithLimit { next, a, b, .. } => {
                let start   = InterceptBlend::LinearFade { a: *a, b: *b }.range().start;
                let finish  = next.range().end;

                start..finish
            }
        }
    }

    ///
    /// Returns the coverage of a particular pixel
    ///
    #[inline]
    pub (crate) fn pixel_coverage(&self, x: f64) -> f64 {
        let mut blend = self;

        loop {
            match blend {
                InterceptBlend::Solid                                       => { break 1.0; },
                InterceptBlend::LinearFade { a, b }                         => { break alpha_coverage(a*x+b, a*(x+1.0)+b); },
                InterceptBlend::LinearFadeWithLimit { a, b, limit, next }   => {
                    if *limit < x {
                        // Starts after this pixel
                        blend = &**next;
                        continue;
                    } else if *limit < x+1.0 {
                        // High frequency case, there are multiple blends to add up
                        let ratio               = *limit-x;
                        let mut total_coverage  = (alpha_coverage(a*x+b, a* *limit+b)) * ratio;
                        let mut last_x          = *limit;

                        // Deal with the other parts covering this pixel
                        blend = &**next;

                        loop {
                            match blend {
                                InterceptBlend::Solid => {
                                    // Remaining part is solid
                                    total_coverage += (x+1.0)-last_x;
                                    break;
                                }

                                InterceptBlend::LinearFade { a, b } => {
                                    // Remaining part has a linear fade
                                    let ratio       = (x+1.0)-last_x;
                                    let coverage    = alpha_coverage(a*last_x+b, a*(x+1.0)+b);

                                    total_coverage  += coverage * ratio;
                                    break;
                                }

                                InterceptBlend::LinearFadeWithLimit { a, b, limit, next } => {
                                    // Blend in this section of the pixel
                                    let pixel_limit = (*limit).max(x+1.0);
                                    let ratio       = pixel_limit - last_x;
                                    let coverage    = alpha_coverage(a*last_x+b, a*pixel_limit+b);

                                    total_coverage += coverage * ratio;

                                    if *limit >= x+1.0 {
                                        break;
                                    }

                                    last_x  = pixel_limit;
                                    blend   = &**next;
                                }
                            }
                        }

                        break total_coverage;
                    } else {
                        // Same as a linear fade
                        break alpha_coverage(a*x+b, a*(x+1.0)+b);
                    }
                }
            }
        }
    }

    ///
    /// Adds a linear fade blend to a pixel program stack
    ///
    #[inline]
    fn render_linear_fade(a: f64, b: f64, program_stack: &mut Vec<PixelProgramPlan>, x_range: &Range<f64>) {
        // For a 'limit' fade, we assume the limit is not hit
        // Convert to a range to use on the program stack
        let x1              = x_range.start.floor();
        let x2              = x_range.end.ceil();
        let initial_fade    = alpha_coverage(a*x1+b, a*(x1+1.0)+b);
        let final_fade      = alpha_coverage(a*(x2-1.0)+b, a*x2+b);

        // TODO: remove this
        #[cfg(debug_assertions)]
        {
            if initial_fade != final_fade {
                let range = range_for_line(a, b);
                debug_assert!(range.start <= x_range.end+1.0, "{:?} {:?} {:?}", range, x_range, (initial_fade, final_fade));
                debug_assert!(range.end >= x_range.start-1.0, "{:?} {:?} {:?}", range, x_range, (initial_fade, final_fade));
            }
        }

        program_stack.push(PixelProgramPlan::LinearMerge(initial_fade as _, final_fade as _));
    }

    ///
    /// Adds this blend to a pixel program stack
    ///
    pub fn render(&self, program_stack: &mut Vec<PixelProgramPlan>, shape_descriptor: &ShapeDescriptor, opacity: f32, x_range: &Range<f64>) {
        let blend           = self;
        let mut num_blends  = 0;

        // Start the blends for the program
        match blend {
            InterceptBlend::Solid => { }

            InterceptBlend::LinearFade { a, b } => {
                Self::render_linear_fade(*a, *b, program_stack, x_range);
                num_blends += 1;
            }

            InterceptBlend::LinearFadeWithLimit { a, b, limit, .. } => {
                let x1 = x_range.start.floor();
                let x2 = x_range.end.ceil();

                if *limit < x1+1.0 && x2 <= x1+1.0 {
                    // Treat as a single pixel with high frequency
                    let initial_blend       = blend;
                    let mut blend           = initial_blend;
                    let mut total_coverage  = 0.0;
                    let mut last_pos        = x1;

                    loop {
                        match blend {
                            InterceptBlend::LinearFade { a, b } => {
                                // Add the coverage until the end of the pixel
                                let pos         = x1+1.0;
                                let last_alpha  = a*last_pos + b;
                                let alpha       = a*pos + b;

                                let coverage    = alpha_coverage(last_alpha, alpha) * (pos-last_pos);
                                total_coverage += coverage;
                                debug_assert!(alpha_coverage(last_alpha, alpha) <= 1.0, "{} > 1.0", alpha_coverage(last_alpha, alpha));

                                break;
                            }

                            InterceptBlend::LinearFadeWithLimit { a, b, limit, next } => {
                                if *limit > last_pos {  // TODO: we should be 'clearing' these before we reach here so this never happens
                                    // Compute the coverage between positions
                                    let pos         = limit.min(x1+1.0);
                                    let last_alpha  = a*last_pos + b;
                                    let alpha       = a*pos + b;

                                    let coverage = alpha_coverage(last_alpha, alpha) * (pos-last_pos);
                                    total_coverage += coverage;
                                    debug_assert!(alpha_coverage(last_alpha, alpha) <= 1.0, "{} > 1.0", alpha_coverage(last_alpha, alpha));

                                    // Stop if this moves beyond the end of the limit
                                    if *limit > x1 + 1.0 {
                                        break;
                                    }

                                    last_pos = pos;
                                }

                                // Add the coverage of the next region
                                blend = &**next;
                            }

                            _ => { break; /* Not expecting the other values */ }
                        }
                    }

                    debug_assert!(total_coverage <= 1.0, "Produced too much alpha: {:?}", initial_blend);
                    program_stack.push(PixelProgramPlan::Merge(total_coverage as _));
                    num_blends += 1;
                } else {
                    // Treat as a 'normal' linear blend
                    Self::render_linear_fade(*a, *b, program_stack, x_range);
                    num_blends += 1;
                }
            },
        }

        // Apply opacity if needed
        if opacity < 1.0 {
            program_stack.push(PixelProgramPlan::Merge(opacity));
            num_blends += 1;
        }

        // Run the program for this range
        program_stack.extend(shape_descriptor.programs.iter().map(|program| PixelProgramPlan::Run(*program)));

        // Finish the blends
        if num_blends > 0 {
            program_stack.extend((0..num_blends).map(|_| PixelProgramPlan::StartBlend));
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn linear_fade() {
        let (a, b) = match InterceptBlend::linear_fade(2.0, 3.0, false) { InterceptBlend::LinearFade { a, b } => (a, b), _ => panic!() };
        assert!(2.0*a + b == 0.0);
        assert!(3.0*a + b == 1.0);

        let (a, b) = match InterceptBlend::linear_fade(3.0, 2.0, true) { InterceptBlend::LinearFade { a, b } => (a, b), _ => panic!() };
        assert!(3.0*a + b == 0.0);
        assert!(2.0*a + b == 1.0);

        let (a, b) = match InterceptBlend::linear_fade(40.0, 900.0, false) { InterceptBlend::LinearFade { a, b } => (a, b), _ => panic!() };
        assert!(((40.0*a + b)-0.0).abs() <= 0.00001);
        assert!(((900.0*a + b)-1.0).abs() <= 0.00001);
    }

    #[test]
    fn linear_fade_range() {
        let range = InterceptBlend::linear_fade(2.0, 3.0, false).range();

        assert!(range.start == 2.0, "{:?}", range);
        assert!(range.end == 3.0, "{:?}", range);

        let range = InterceptBlend::linear_fade(3.0, 2.0, false).range();

        assert!(range.start == 2.0, "{:?}", range);
        assert!(range.end == 3.0, "{:?}", range);
    }

    #[test]
    fn linear_fade_multiply() {
        let (a, b) = match InterceptBlend::linear_fade(2.0, 3.0, false).multiply_fade(0.5) 
            { InterceptBlend::LinearFade { a, b } => (a, b), _ => panic!() };
        assert!(2.0*a + b == 0.0);
        assert!(3.0*a + b == 0.5);
    }

    #[test]
    fn linear_fade_multiply_simple_nest() {
        let (a, b) = match InterceptBlend::linear_fade(2.0, 3.0, false).multiply_fade(0.5).nest(InterceptBlend::linear_fade(2.0, 3.0, false).multiply_fade(0.5))
            { InterceptBlend::LinearFade { a, b } => (a, b), _ => panic!() };
        assert!(2.0*a + b == 0.0);
        assert!(3.0*a + b == 1.0);
    }

    fn blend_factor(blend: &InterceptBlend, x_pos: f64) -> f64 {
        match blend {
            InterceptBlend::Solid                                     => 1.0,
            InterceptBlend::LinearFade { a, b }                       => a*x_pos + b,
            InterceptBlend::LinearFadeWithLimit { a, b, limit, next } => {
                if x_pos < *limit {
                    a*x_pos + b
                } else {
                    blend_factor(&**next, x_pos)
                }
            },
        }
    }

    #[test]
    fn split_fading_in_same_origin() {
        let blend1 = InterceptBlend::linear_fade(1.0, 3.0, false);
        let blend2 = InterceptBlend::linear_fade(1.0, 4.0, false);
        let nested = blend1.nest(blend2.clone());

        // The initial section should combine the two blends, then the next section should be solid, as blend1 is saturated
        let blend_at_one    = blend_factor(&nested, 1.0);
        let blend_at_three  = blend_factor(&nested, 3.0-1e-6);

        // Calculate the expected values by adding the blends
        let expected_at_one     = blend_factor(&blend1, 1.0) + blend_factor(&blend2, 1.0);
        let expected_at_three   = blend_factor(&blend1, 3.0) + blend_factor(&blend2, 3.0);

        // Check the values
        assert!((blend_at_three-expected_at_three).abs() < 1e-6, "f(1.0) = {:?} f(3.0) = {:?} (!= {:?})", blend_at_one, blend_at_three, expected_at_three);
        assert!((blend_at_one-expected_at_one).abs() < 1e-6,     "f(1.0) = {:?} (!= {:?}) f(3.0) = {:?}", blend_at_one, expected_at_one, blend_at_three);
    }

    #[test]
    fn split_fading_out_same_origin() {
        let blend1 = InterceptBlend::linear_fade(3.0, 1.0, true);
        let blend2 = InterceptBlend::linear_fade(4.0, 1.0, true);
        let nested = blend1.nest(blend2.clone());

        // The initial section should combine the two blends, then the next section should be just blend2
        let blend_at_one    = blend_factor(&nested, 1.0);
        let blend_at_three  = blend_factor(&nested, 3.0-1e-6);

        // Calculate the expected values by adding the blends
        let expected_at_one     = blend_factor(&blend1, 1.0) + blend_factor(&blend2, 1.0);
        let expected_at_three   = blend_factor(&blend1, 3.0) + blend_factor(&blend2, 3.0);

        // Check the values
        assert!((blend_at_three-expected_at_three).abs() < 1e-6, "f(1.0) = {:?} f(3.0) = {:?} (!= {:?})", blend_at_one, blend_at_three, expected_at_three);
        assert!((blend_at_one-expected_at_one).abs() < 1e-6,     "f(1.0) = {:?} (!= {:?}) f(3.0) = {:?}", blend_at_one, expected_at_one, blend_at_three);
    }

    fn check_blend_order(blend: &InterceptBlend, last_limit: f64, root_blend: &InterceptBlend) {
        match blend {
            InterceptBlend::LinearFadeWithLimit { limit, next, .. } => {
                assert!(*limit > last_limit, "{:?} should be greated than {:?} in blend {:?}", limit, last_limit, root_blend);
                check_blend_order(&**next, *limit, root_blend);
            }

            _ => { }
        }
    }

    #[test]
    fn low_frequency_thin_vertical_line_0point5() {
        use smallvec::*;

        // Half pixel filled, half clear
        let blend               = InterceptBlend::linear_fade(2.5, 2.5+1e-6, false);
        let shape_descriptor    = ShapeDescriptor { programs: smallvec![], is_opaque: true, z_index: 0 };
        let mut program_stack   = vec![];

        blend.render(&mut program_stack, &shape_descriptor, 1.0, &(2.0..3.0));

        assert!(program_stack.len() == 2, "{:?}.len() != 2", program_stack);
        assert!(program_stack[1] == PixelProgramPlan::StartBlend, "{:?}[1] != StartBlend", program_stack[1]);
        assert!(match &program_stack[0] { 
            PixelProgramPlan::LinearMerge(amount1, amount2) => (amount1-0.5).abs() < 1e-5 && (amount2-0.5).abs() < 1e-5,
            _ => false
        }, "{:?}[0] != LinearMerge(0.5, 0.5)", program_stack);

        check_blend_order(&blend, 0.0, &blend);
    }

    #[test]
    fn high_frequency_thin_vertical_line_0point5() {
        use smallvec::*;

        // Create a blend that's the equivalent of a <1 px line in the center of a pixel
        let blend               = InterceptBlend::linear_fade(2.25, 2.25, false).nest(InterceptBlend::linear_fade(2.75, 2.75, true));
        let shape_descriptor    = ShapeDescriptor { programs: smallvec![], is_opaque: true, z_index: 0 };
        let mut program_stack   = vec![];

        println!("{:?}", blend);
        blend.render(&mut program_stack, &shape_descriptor, 1.0, &(2.0..3.0));

        assert!(program_stack.len() == 2, "{:?}.len() != 2", program_stack);
        assert!(program_stack[1] == PixelProgramPlan::StartBlend, "{:?}[1] != StartBlend", program_stack[1]);
        assert!(match &program_stack[0] { 
            PixelProgramPlan::Merge(amount) => (amount-0.5).abs() < 1e-5,
            _ => false
        }, "{:?}[0] != Merge(0.5)", program_stack);

        check_blend_order(&blend, 0.0, &blend);
    }

    #[test]
    fn high_frequency_thin_vertical_line_0point5_2() {
        use smallvec::*;

        // Create a blend that's the equivalent of a <1 px line in the center of a pixel
        let blend               = InterceptBlend::linear_fade(2.0, 2.0, false).nest(InterceptBlend::linear_fade(2.5, 2.5, true));
        let shape_descriptor    = ShapeDescriptor { programs: smallvec![], is_opaque: true, z_index: 0 };
        let mut program_stack   = vec![];

        blend.render(&mut program_stack, &shape_descriptor, 1.0, &(2.0..3.0));

        check_blend_order(&blend, 0.0, &blend);

        assert!(program_stack.len() == 2, "{:?}.len() != 2", program_stack);
        assert!(program_stack[1] == PixelProgramPlan::StartBlend, "{:?}[1] != StartBlend", program_stack[1]);
        assert!(match &program_stack[0] { 
            PixelProgramPlan::Merge(amount) => (amount-0.5).abs() < 1e-5,
            _ => false
        }, "{:?}[0] != Merge(0.5)", program_stack);
    }

    #[test]
    fn high_frequency_thin_vertical_line_0point5_2b() {
        use smallvec::*;

        // Create a blend that's the equivalent of a <1 px line in the center of a pixel (this uses nearly but not quite vertical lines)
        // (Note that 'fade in' and 'fade out' involve lines sloping in different directions)
        let blend               = InterceptBlend::linear_fade(2.0, 2.0+1e-6, false).nest(InterceptBlend::linear_fade(2.5+1e-6, 2.5, true));
        let shape_descriptor    = ShapeDescriptor { programs: smallvec![], is_opaque: true, z_index: 0 };
        let mut program_stack   = vec![];

        blend.render(&mut program_stack, &shape_descriptor, 1.0, &(2.0..3.0));

        assert!(program_stack.len() == 2, "{:?}.len() != 2", program_stack);
        assert!(program_stack[1] == PixelProgramPlan::StartBlend, "{:?}[1] != StartBlend", program_stack[1]);
        assert!(match &program_stack[0] { 
            PixelProgramPlan::Merge(amount) => (amount-0.5).abs() < 1e-5,
            _ => false
        }, "{:?}[0] != Merge(0.5)", program_stack);

        check_blend_order(&blend, 0.0, &blend);
    }

    #[test]
    fn high_frequency_thin_vertical_line_0point5_3() {
        use smallvec::*;

        // Create a blend that's the equivalent of a <1 px line in the center of a pixel
        let blend               = InterceptBlend::linear_fade(2.5, 2.5, false).nest(InterceptBlend::linear_fade(3.0, 3.0, true));
        let shape_descriptor    = ShapeDescriptor { programs: smallvec![], is_opaque: true, z_index: 0 };
        let mut program_stack   = vec![];

        blend.render(&mut program_stack, &shape_descriptor, 1.0, &(2.0..3.0));

        check_blend_order(&blend, 0.0, &blend);

        assert!(program_stack.len() == 2, "{:?}.len() != 2", program_stack);
        assert!(program_stack[1] == PixelProgramPlan::StartBlend, "{:?}[1] != StartBlend", program_stack[1]);
        assert!(match &program_stack[0] { 
            PixelProgramPlan::LinearMerge(amount1, amount2) => (amount1-0.5).abs() < 1e-5 && (amount2-0.5).abs() < 1e-5,
            _ => false
        }, "{:?}[0] != LinearMerge(0.5, 0.5)", program_stack);
    }

    #[test]
    fn high_frequency_thin_vertical_line_0point5_4() {
        use smallvec::*;

        // Create a blend that's from two vertical lines crossing one pixel (with a total coverage of 50%)
        let blend               = InterceptBlend::linear_fade(2.0, 2.0, false)
            .nest(InterceptBlend::linear_fade(2.25, 2.25, true))
            .nest(InterceptBlend::linear_fade(2.75, 2.75, false))
            .nest(InterceptBlend::linear_fade(3.0, 3.0, true));
        let shape_descriptor    = ShapeDescriptor { programs: smallvec![], is_opaque: true, z_index: 0 };
        let mut program_stack   = vec![];

        blend.render(&mut program_stack, &shape_descriptor, 1.0, &(2.0..3.0));

        check_blend_order(&blend, 0.0, &blend);

        assert!(program_stack.len() == 2, "{:?}.len() != 2", program_stack);
        assert!(program_stack[1] == PixelProgramPlan::StartBlend, "{:?}[1] != StartBlend", program_stack[1]);
        assert!(match &program_stack[0] { 
            PixelProgramPlan::Merge(amount) => (amount-0.5).abs() < 1e-5,
            _ => false
        }, "{:?}[0] != Merge(0.5)", program_stack);
    }

    #[test]
    fn high_frequency_thin_vertical_line_0point5_4b() {
        use smallvec::*;

        // Create a blend that's from two vertical lines crossing one pixel (with a total coverage of 50%)
        let blend               = InterceptBlend::linear_fade(2.0, 2.0+1e-6, true)
            .nest(InterceptBlend::linear_fade(2.25+1e-6, 2.25, false))
            .nest(InterceptBlend::linear_fade(2.75, 2.75+1e-6, true))
            .nest(InterceptBlend::linear_fade(3.0+1e-6, 3.0, false));
        let shape_descriptor    = ShapeDescriptor { programs: smallvec![], is_opaque: true, z_index: 0 };
        let mut program_stack   = vec![];

        blend.render(&mut program_stack, &shape_descriptor, 1.0, &(2.0..3.0));

        assert!(program_stack.len() == 2, "{:?}.len() != 2", program_stack);
        assert!(program_stack[1] == PixelProgramPlan::StartBlend, "{:?}[1] != StartBlend", program_stack[1]);
        assert!(match &program_stack[0] { 
            PixelProgramPlan::Merge(amount) => (amount-0.5).abs() < 1e-5,
            _ => false
        }, "{:?}[0] != Merge(0.5)", program_stack);
    }

    #[test]
    fn high_frequency_thin_vertical_line_0point25() {
        use smallvec::*;

        // Create a blend that's the equivalent of a <1 px line in the center of a pixel
        let blend               = InterceptBlend::linear_fade(2.375, 2.375, false).nest(InterceptBlend::linear_fade(2.625, 2.625, true));
        let shape_descriptor    = ShapeDescriptor { programs: smallvec![], is_opaque: true, z_index: 0 };
        let mut program_stack   = vec![];

        println!("{:?}", blend);

        blend.render(&mut program_stack, &shape_descriptor, 1.0, &(2.0..3.0));

        check_blend_order(&blend, 0.0, &blend);

        assert!(program_stack.len() == 2, "{:?}.len() != 2", program_stack);
        assert!(program_stack[1] == PixelProgramPlan::StartBlend, "{:?}[1] != StartBlend", program_stack[1]);
        assert!(match &program_stack[0] { 
            PixelProgramPlan::Merge(amount) => (amount-0.25).abs() < 1e-5,
            _ => false
        }, "{:?}[0] != Merge(0.25)", program_stack);
    }

    #[test]
    fn high_frequency_thin_vertical_line_0point25b() {
        use smallvec::*;

        // Create a blend that's the equivalent of a <1 px line in the center of a pixel
        let blend               = InterceptBlend::linear_fade(2.375, 2.375+1e-6, false).nest(InterceptBlend::linear_fade(2.625+1e-6, 2.625, true));
        let shape_descriptor    = ShapeDescriptor { programs: smallvec![], is_opaque: true, z_index: 0 };
        let mut program_stack   = vec![];

        println!("{:?}", blend);

        blend.render(&mut program_stack, &shape_descriptor, 1.0, &(2.0..3.0));

        assert!(program_stack.len() == 2, "{:?}.len() != 2", program_stack);
        assert!(program_stack[1] == PixelProgramPlan::StartBlend, "{:?}[1] != StartBlend", program_stack[1]);
        assert!(match &program_stack[0] { 
            PixelProgramPlan::Merge(amount) => (amount-0.25).abs() < 1e-5,
            _ => false
        }, "{:?}[0] != Merge(0.25)", program_stack);
    }

    #[test]
    fn nest_linear_fades_with_limit_1() {
        let first   = InterceptBlend::LinearFadeWithLimit { a: 0.1, b: 0.0, limit: 3.0, next: Box::new(InterceptBlend::LinearFade { a: (-0.7/-3.0), b: -0.4 }) };
        let second  = InterceptBlend::LinearFadeWithLimit { a: 0.25, b: 0.0, limit: 1.0, next: Box::new(InterceptBlend::LinearFade { a: 0.75/19.0, b: 0.25-(0.75/19.0) }) };
        let nested  = first.nest(second.clone());

        let test_points     = [ 0.0, 0.1, 0.9, 1.0, 1.5, 2.9, 3.0, 3.1, 4.0, 5.0 ];
        let our_values      = test_points.iter().map(|val| blend_factor(&nested, *val)).collect::<Vec<_>>();
        let their_values    = test_points.iter().map(|val| blend_factor(&first, *val) + blend_factor(&second, *val)).collect::<Vec<_>>();

        println!("{:?}\n\nActual: {:?}\n\nExpected: {:?}", nested, our_values, their_values);

        for (actual, expected) in our_values.iter().zip(their_values.iter()) {
            assert!((actual-expected).abs() < 0.01, "{} != {}", actual, expected);
        }
    }

    #[test]
    fn nest_linear_fades_with_limit_2() {
        let first   = InterceptBlend::LinearFadeWithLimit { a: 0.25, b: 0.0, limit: 1.0, next: Box::new(InterceptBlend::LinearFade { a: 0.75/19.0, b: 0.25-(0.75/19.0) }) };
        let second  = InterceptBlend::LinearFadeWithLimit { a: 0.1, b: 0.0, limit: 3.0, next: Box::new(InterceptBlend::LinearFade { a: (-0.7/-3.0), b: -0.4 }) };
        let nested  = first.nest(second.clone());

        let test_points     = [ 0.0, 0.1, 0.9, 1.0, 1.5, 2.9, 3.0, 3.1, 4.0, 5.0 ];
        let our_values      = test_points.iter().map(|val| blend_factor(&nested, *val)).collect::<Vec<_>>();
        let their_values    = test_points.iter().map(|val| blend_factor(&first, *val) + blend_factor(&second, *val)).collect::<Vec<_>>();

        println!("{:?}\n\nActual: {:?}\n\nExpected: {:?}", nested, our_values, their_values);

        for (actual, expected) in our_values.iter().zip(their_values.iter()) {
            assert!((actual-expected).abs() < 0.01, "{} != {}", actual, expected);
        }
    }
}
