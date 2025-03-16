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

    /// This should be alpha-blended using source-over with a linear fade
    Fade { x_range: Range<f64>, alpha_range: Range<f64> },

    /// Nest the second blend inside the first blend
    NestedFade { x_range: Range<f64>, alpha_range: Range<f64>, nested: Box<InterceptBlend>, },
}

impl InterceptBlend {
    ///
    /// Creates a linear fade that has an alpha of 0 at zero_x and 1 at one_x
    ///
    #[inline]
    pub fn linear_fade(zero_x: f64, one_x: f64) -> InterceptBlend {
        let a = 1.0/(one_x-zero_x);
        let b = 1.0-a*one_x;

        InterceptBlend::LinearFade { a, b }
    }

    ///
    /// If this is a fade blend, multiply the ratios by the specified number
    ///
    pub fn multiply_fade(&self, factor: f64) -> InterceptBlend {
        match self {
            InterceptBlend::Solid                                       => InterceptBlend::Solid,
            InterceptBlend::LinearFade { a, b }                         => InterceptBlend::LinearFade { a: a*factor, b: b*factor },
            InterceptBlend::Fade { x_range, alpha_range }               => InterceptBlend::Fade { x_range: x_range.clone(), alpha_range: (alpha_range.start*factor)..(alpha_range.end*factor) },
            InterceptBlend::NestedFade { x_range, alpha_range, nested } => InterceptBlend::NestedFade { x_range: x_range.clone(), alpha_range: (alpha_range.start*factor)..(alpha_range.end*factor), nested: Box::new(nested.multiply_fade(factor)) },
        }
    }

    ///
    /// Nests another fade blend inside this one
    ///
    pub fn nest(&self, blend: InterceptBlend) -> InterceptBlend {
        match self {
            InterceptBlend::Solid                                       => InterceptBlend::Solid,
            InterceptBlend::LinearFade { a, b }                         => todo!(),
            InterceptBlend::Fade { x_range, alpha_range }               => InterceptBlend::NestedFade { x_range: x_range.clone(), alpha_range: alpha_range.clone(), nested: Box::new(blend) },
            InterceptBlend::NestedFade { x_range, alpha_range, nested } => InterceptBlend::NestedFade { x_range: x_range.clone(), alpha_range: alpha_range.clone(), nested: Box::new(nested.nest(blend)) }
        }
    }

    ///
    /// Adds this blend to a pixel program stack
    ///
    pub fn render(&self, program_stack: &mut Vec<PixelProgramPlan>, shape_descriptor: &ShapeDescriptor, opacity: f32, x_range: &Range<f64>) {
        let mut blend       = self;
        let mut num_blends  = 0;

        // Start the blends for the program
        loop {
            match blend {
                InterceptBlend::Solid => {
                    break;
                },

                InterceptBlend::LinearFade { a, b } => todo!(),

                InterceptBlend::Fade { x_range: alpha_x_range, alpha_range } => {
                    // Adjust the alpha range to the actual x range
                    let corrected_range = actual_fade_for_range(&x_range, &alpha_x_range, &alpha_range);

                    // Run a linear blend using the corrected range
                    program_stack.push(PixelProgramPlan::LinearMerge(corrected_range.start as _, corrected_range.end as _));
                    num_blends += 1;
                    break;
                },

                InterceptBlend::NestedFade { x_range: alpha_x_range, alpha_range, nested } => {
                    // Adjust the alpha range to the actual x range
                    let corrected_range = actual_fade_for_range(&x_range, &alpha_x_range, &alpha_range);

                    // Run a linear blend using the corrected range
                    program_stack.push(PixelProgramPlan::LinearMerge(corrected_range.start as _, corrected_range.end as _));

                    // Apply the nested gradient as well
                    blend       = &*nested;
                    num_blends += 1;
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
}
