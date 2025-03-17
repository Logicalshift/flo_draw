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
    LinearFadeWithLimit { a: f64, b: f64, limit: f64, next: Box<InterceptBlend> }
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
/// Given two linear fades, creates a 'split' blend (at the point that one fade reaches 0 or 1)
///
fn split(a1: f64, b1: f64, a2: f64, b2: f64) -> InterceptBlend {
    let range1 = range_for_line(a1, b1);
    let range2 = range_for_line(a2, b2);

    // End points are either 0 or 1
    let end1 = a1*range1.end + b1;
    let end2 = a2*range2.end + b2;

    if range2.end == range1.end {
        // Rare: both end at the same point
        InterceptBlend::LinearFade { a: a1+a2, b: b1+b2 }
    } else if range2.end > range1.end {
        // a1, b1 ends first
        if end1 < 0.5 {
            // a1, b1 blends to 0 before a2, b2 completes (after that point, we just follow a2, b2)
            InterceptBlend::LinearFadeWithLimit { 
                a: a1+a2, b: b1+b2, limit: end1, 
                next: Box::new(InterceptBlend::LinearFade { a: a2, b: b2 }) 
            }
        } else {
            // a1, b1 blends to 1 before a2, b2 completes, is saturated after this point
            InterceptBlend::LinearFade { a: a1+a2, b: b1+b2 }
        }
    } else {
        // a2, b2 ends first
        if end2 < 0.5 {
            // a2, b2 blends to 0 before a1, b1 completes (after that point, we just follow a1, b1)
            InterceptBlend::LinearFadeWithLimit { 
                a: a1+a2, b: b1+b2, limit: end2, 
                next: Box::new(InterceptBlend::LinearFade { a: a1, b: b1 }) 
            }
        } else {
            // a2, b2 blends to 1 before a1, b1 completes, is saturated after this point
            InterceptBlend::LinearFade { a: a1+a2, b: b1+b2 }
        }
    }
}

impl InterceptBlend {
    ///
    /// Creates a linear fade that has an alpha of 0 at zero_x and 1 at one_x
    ///
    #[inline]
    pub fn linear_fade(zero_x: f64, one_x: f64) -> InterceptBlend {
        if zero_x == one_x {
            InterceptBlend::Solid
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
            InterceptBlend::LinearFadeWithLimit { a, b, limit, next }   => InterceptBlend::LinearFadeWithLimit { a: a*factor, b: b*factor, limit: *limit, next: Box::new(next.multiply_fade(factor)) }
        }
    }

    ///
    /// Nests another fade blend inside this one
    ///
    pub fn nest(&self, blend: InterceptBlend) -> InterceptBlend {
        match self {
            InterceptBlend::Solid                                               => InterceptBlend::Solid,
            InterceptBlend::LinearFade { a, b }                                 => {
                match &blend {
                    InterceptBlend::Solid                                       => InterceptBlend::Solid,
                    InterceptBlend::LinearFade { a: a2, b: b2 }                 => split(*a, *b, *a2, *b2),
                    InterceptBlend::LinearFadeWithLimit { a: a2, b: b2, limit, next }   => {
                        blend.clone() // TODO
                    }
                }
            },

            InterceptBlend::LinearFadeWithLimit { a, b, limit, next } => {
                match blend {
                    InterceptBlend::Solid   => InterceptBlend::Solid,
                    _                       => blend.clone() // TODO
                }
            }
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
    /// Adds this blend to a pixel program stack
    ///
    pub fn render(&self, program_stack: &mut Vec<PixelProgramPlan>, shape_descriptor: &ShapeDescriptor, opacity: f32, x_range: &Range<f64>) {
        let blend           = self;
        let mut num_blends  = 0;

        // Start the blends for the program
        loop {
            match blend {
                InterceptBlend::Solid => {
                    break;
                },

                InterceptBlend::LinearFade { a, b } |
                InterceptBlend::LinearFadeWithLimit { a, b, .. } => {
                    // For a 'limit' fade, we assume the limit is not hit
                    // Convert to a range to use on the program stack
                    let x1              = x_range.start.floor();
                    let x2              = x_range.end.floor();
                    let initial_fade    = alpha_coverage(a*x1+b, a*(x1+1.0)+b);
                    let final_fade      = alpha_coverage(a*x2+b, a*(x2+1.0)+b);

                    program_stack.push(PixelProgramPlan::LinearMerge(initial_fade as _, final_fade as _));
                    num_blends += 1;
                    break;
                },
            }
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

///
/// Calculates the range of alpha values to use in a rendering program.
///
/// The `render_x_range` is the range of values where the pixels will be rendered. The alpha_x_range is the full range of the fade,
/// and the alpha_range is the range of alpha values that will be covered.
///
/// This will the alpha values at the start and end of the pixels to calculate the initial and final coverage of the range (ie, if
/// the alpha value is 0 at the start of a pixel and non-zero at the end, that pixel will be partially covered rather than clear)
///
#[inline]
fn actual_fade_for_range(render_x_range: &Range<f64>, alpha_x_range: &Range<f64>, alpha_range: &Range<f64>) -> Range<f64> {
    if (alpha_x_range.end - alpha_x_range.start).abs() < 1e-6 {
        // Treat the intercept as a vertical line
        let offset = alpha_x_range.start - render_x_range.start;
        let offset = offset / (render_x_range.end - render_x_range.start);

        // Assuming that we're covering a single pixel, we can just use the offset here
        if alpha_range.start > 0.5 {
            offset..offset
        } else {
            (1.0-offset)..(1.0-offset)
        }
    } else {
        // Adjust the alpha range to the actual x range
        let pixel_x_start   = render_x_range.start.floor();
        let pixel_x_end     = render_x_range.end.ceil();
        let alpha_x_start   = alpha_x_range.start;
        let alpha_x_end     = alpha_x_range.end;

        let alpha_ratio     = (alpha_range.end - alpha_range.start) / (alpha_x_end-alpha_x_start);

        let corrected_start_1   = (pixel_x_start - alpha_x_start) * alpha_ratio + alpha_range.start;
        let corrected_start_2   = ((pixel_x_start + 1.0) - alpha_x_start) * alpha_ratio + alpha_range.start;
        let corrected_end_1     = ((pixel_x_end - 1.0) - alpha_x_start) * alpha_ratio + alpha_range.start;
        let corrected_end_2     = (pixel_x_end - alpha_x_start) * alpha_ratio + alpha_range.start;

        let corrected_start     = alpha_coverage(corrected_start_1, corrected_start_2);
        let corrected_end       = alpha_coverage(corrected_end_1, corrected_end_2);

        let corrected_range = (corrected_start.max(0.0).min(1.0))..(corrected_end.max(0.0).min(1.0));

        corrected_range
    }
}

///
/// Returns the coverage for a pixel. The two values passed in are the alpha value at the start of the
/// pixel (ie, where x=0) and the end (x=1).
///
fn alpha_coverage(pixel_start: f64, pixel_end: f64) -> f64 {
    // ax = 0, bx = 1
    let ay = pixel_start.min(pixel_end);
    let by = pixel_end.max(pixel_start);

    if ay <= 0.0 && by <= 0.0 {
        // Both outside
        0.0
    } else if ay >= 1.0 && by >= 1.0 {
        // Whole pixel covered
        1.0
    } else if ay < 0.0 {
        // Calculate intercept on the x-axis (y = 0)
        let cx = -ay/(by-ay);

        if by < 1.0 {
            // Crosses the x-axis then the y-axis
            0.5 * (1.0-cx) * by
        } else {
            // Crosses the x-axis twice

            // Calculate intercept on x-axis for y=1
            let dx = (1.0-ay)/(by-ay);

            0.5 * (dx-cx) + (1.0-dx)
        }
    } else {
        // LHS intercepts the y-axis
        if by < 1.0 {
            // Both sides intercept the y-axis
            0.5 * (by-ay) + ay
        } else {
            // Calculate intercept on x-axis for y=1
            let dx = (1.0-ay)/(by-ay);

            1.0 - (0.5 * dx * (1.0 - ay))
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn alpha_coverage_100_percent() {
        let coverage = alpha_coverage(1.5, 1.5);
        assert!((coverage-1.0).abs() < 0.0001, "{:?}", coverage);
    }

    #[test]
    fn alpha_coverage_0_percent() {
        let coverage = alpha_coverage(-0.5, -0.5);
        assert!((coverage-0.0).abs() < 0.0001, "{:?}", coverage);
    }

    #[test]
    fn alpha_coverage_50_percent() {
        let coverage = alpha_coverage(0.0, 1.0);
        assert!((coverage-0.5).abs() < 0.0001, "{:?}", coverage);
    }

    #[test]
    fn alpha_coverage_lower_eighth() {
        let coverage = alpha_coverage(-0.5, 0.5);
        assert!((coverage-0.125).abs() < 0.0001, "{:?}", coverage);
    }

    #[test]
    fn alpha_coverage_lower_quarter() {
        let coverage = alpha_coverage(0.0, 0.5);
        assert!((coverage-0.25).abs() < 0.0001, "{:?}", coverage);
    }

    #[test]
    fn alpha_coverage_upper_quarter() {
        let coverage = alpha_coverage(0.5, 1.0);
        assert!((coverage-0.75).abs() < 0.0001, "{:?}", coverage);
    }

    #[test]
    fn alpha_coverage_lower_quarter_reversed() {
        let coverage = alpha_coverage(0.5, 0.0);
        assert!((coverage-0.25).abs() < 0.0001, "{:?}", coverage);
    }

    #[test]
    fn alpha_coverage_lower_partial() {
        let coverage = alpha_coverage(0.1, 0.6);
        assert!((coverage-0.35).abs() < 0.0001, "{:?}", coverage);
    }

    #[test]
    fn alpha_coverage_middle_quarter() {
        let coverage = alpha_coverage(-0.5, 2.0);
        assert!((coverage-0.6).abs() < 0.0001, "{:?}", coverage);
    }

    #[test]
    fn linear_fade() {
        let (a, b) = match InterceptBlend::linear_fade(2.0, 3.0) { InterceptBlend::LinearFade { a, b } => (a, b), _ => panic!() };
        assert!(2.0*a + b == 0.0);
        assert!(3.0*a + b == 1.0);

        let (a, b) = match InterceptBlend::linear_fade(3.0, 2.0) { InterceptBlend::LinearFade { a, b } => (a, b), _ => panic!() };
        assert!(3.0*a + b == 0.0);
        assert!(2.0*a + b == 1.0);

        let (a, b) = match InterceptBlend::linear_fade(40.0, 900.0) { InterceptBlend::LinearFade { a, b } => (a, b), _ => panic!() };
        assert!(((40.0*a + b)-0.0).abs() <= 0.00001);
        assert!(((900.0*a + b)-1.0).abs() <= 0.00001);
    }

    #[test]
    fn linear_fade_range() {
        let range = InterceptBlend::linear_fade(2.0, 3.0).range();

        assert!(range.start == 2.0, "{:?}", range);
        assert!(range.end == 3.0, "{:?}", range);

        let range = InterceptBlend::linear_fade(3.0, 2.0).range();

        assert!(range.start == 2.0, "{:?}", range);
        assert!(range.end == 3.0, "{:?}", range);
    }

    #[test]
    fn linear_fade_multiply() {
        let (a, b) = match InterceptBlend::linear_fade(2.0, 3.0).multiply_fade(0.5) 
            { InterceptBlend::LinearFade { a, b } => (a, b), _ => panic!() };
        assert!(2.0*a + b == 0.0);
        assert!(3.0*a + b == 0.5);
    }

    #[test]
    fn linear_fade_multiply_simple_nest() {
        let (a, b) = match InterceptBlend::linear_fade(2.0, 3.0).multiply_fade(0.5).nest(InterceptBlend::linear_fade(2.0, 3.0).multiply_fade(0.5))
            { InterceptBlend::LinearFade { a, b } => (a, b), _ => panic!() };
        assert!(2.0*a + b == 0.0);
        assert!(3.0*a + b == 1.0);
    }
}
