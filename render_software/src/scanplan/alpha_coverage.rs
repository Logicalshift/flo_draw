///
/// Returns the coverage for a pixel. The two values passed in are the alpha value at the start of the
/// pixel (ie, where x=0) and the end (x=1).
///
pub fn alpha_coverage(pixel_start: f64, pixel_end: f64) -> f64 {
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
}
