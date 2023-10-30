use std::ops::{Range};

///
/// Describes how to transform the x positions in the edge plan to the viewport x positions
///
#[derive(Clone, Copy)]
pub struct ScanlineTransform {
    /// Value to add to the x coordinates before transforming
    offset: f64,

    /// The scale to apply to the x coordinate to convert it to pixel coordinates
    scale: f64,

    /// The reciprocal of the scale
    scale_recip: f64,
}

impl ScanlineTransform {
    ///
    /// Creates an identity transform (where pixel coordinates map directly on to the edge plan)
    ///
    #[inline]
    pub fn identity() -> Self {
        ScanlineTransform { 
            offset:         0.0, 
            scale:          1.0, 
            scale_recip:    1.0
        }
    }

    ///
    /// Creates a scanline transform that maps from the specified source x range to pixel values of 0..pixel_width
    ///
    #[inline]
    pub fn for_region(source_x_range: &Range<f64>, pixel_width: usize) -> Self {
        ScanlineTransform {
            offset:         -source_x_range.start,
            scale:          (pixel_width as f64) / (source_x_range.end-source_x_range.start),
            scale_recip:    (source_x_range.end-source_x_range.start) / (pixel_width as f64),
        }
    }

    ///
    /// Creates a scaled/translated version of the scanline transform
    ///
    #[inline]
    pub fn transform(&self, scale_x: f64, translate_x: f64) -> Self {
        ScanlineTransform { 
            offset:         (self.offset*scale_x) - translate_x, 
            scale:          self.scale / scale_x, 
            scale_recip:    self.scale_recip * scale_x,
        }
    }

    ///
    /// Converts an x-position from the source to pixels
    ///
    #[inline]
    pub fn source_x_to_pixels(&self, source_x: f64) -> f64 {
        (source_x + self.offset) * self.scale
    }

    ///
    /// Converts a range in pixel coordinates to source coordinates
    ///
    #[inline]
    pub fn pixel_x_to_source_x(&self, pixel_x: i32) -> f64 {
        ((pixel_x as f64) * self.scale_recip) - self.offset
    }

    ///
    /// Converts a range in pixel coordinates to source coordinates
    ///
    #[inline]
    pub fn pixel_range_to_x(&self, pixels: &Range<i32>) -> Range<f64> {
        self.pixel_x_to_source_x(pixels.start)..self.pixel_x_to_source_x(pixels.end)
    }

    ///
    /// The size of a pixel in source coordinates
    ///
    #[inline]
    pub fn pixel_size(&self) -> f64 {
        self.scale_recip
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn transform_convert_start_to_pixels() {
        let transform = ScanlineTransform::for_region(&(-1.0..1.0), 1000);
        let converted = transform.source_x_to_pixels(-1.0);

        assert!((converted-0.0).abs() < 0.1, "Expected 0, got {}", converted);
    }

    #[test]
    fn transform_convert_start_from_pixels() {
        let transform = ScanlineTransform::for_region(&(-1.0..1.0), 1000);
        let converted = transform.pixel_x_to_source_x(0);

        assert!((converted--1.0).abs() < 0.1, "Expected -1.0, got {}", converted);
    }

    #[test]
    fn transform_convert_end_to_pixels() {
        let transform = ScanlineTransform::for_region(&(-1.0..1.0), 1000);
        let converted = transform.source_x_to_pixels(1.0);

        assert!((converted-1000.0).abs() < 0.1, "Expected 1000, got {}", converted);
    }

    #[test]
    fn transform_convert_end_from_pixels() {
        let transform = ScanlineTransform::for_region(&(-1.0..1.0), 1000);
        let converted = transform.pixel_x_to_source_x(1000);

        assert!((converted-1.0).abs() < 0.1, "Expected -1.0, got {}", converted);
    }

    #[test]
    fn transform_convert_middle_to_pixels() {
        let transform = ScanlineTransform::for_region(&(-1.0..1.0), 1000);
        let converted = transform.source_x_to_pixels(0.0);

        assert!((converted-500.0).abs() < 0.1, "Expected 500, got {}", converted);
    }

    #[test]
    fn transform_convert_pixels() {
        let transform = ScanlineTransform::for_region(&(-1.0..1.0), 1000);
        let converted = transform.pixel_range_to_x(&(250..750));

        assert!((converted.start- -0.5).abs() < 0.01, "Expected -0.5, got {:?}", converted);
        assert!((converted.end-0.5).abs() < 0.01, "Expected 0.5, got {:?}", converted);
    }

    #[test]
    fn transform_scale_1() {
        let transformed     = ScanlineTransform::for_region(&(-1.0..1.0), 1000).transform(2.0, 0.0);
        let converted_left  = transformed.pixel_x_to_source_x(0);
        let converted_right = transformed.pixel_x_to_source_x(1000);

        assert!((converted_left-(-1.0 * 2.0 + 0.0)).abs() < 0.01, "Expected {}, got {}", (-1.0 * 2.0 + 0.0), converted_left);
        assert!((converted_right-(1.0 * 2.0 + 0.0)).abs() < 0.01, "Expected {}, got {}", (1.0 * 2.0 + 0.0), converted_right);
    }

    #[test]
    fn transform_scale_2() {
        let transformed     = ScanlineTransform::for_region(&(-1.0..1.0), 1000).transform(2.0, 0.0);
        let converted_left  = transformed.source_x_to_pixels(-1.0 * 2.0 + 0.0);
        let converted_right = transformed.source_x_to_pixels(1.0 * 2.0 + 0.0);

        assert!((converted_left-(0.0)).abs() < 0.01, "Expected {}, got {}", 0.0, converted_left);
        assert!((converted_right-(1000.0)).abs() < 0.01, "Expected {}, got {}", 1000.0, converted_right);
    }

    #[test]
    fn transform_translate_1() {
        let transformed     = ScanlineTransform::for_region(&(-1.0..1.0), 1000).transform(1.0, 2.0);
        let converted_left  = transformed.pixel_x_to_source_x(0);
        let converted_right = transformed.pixel_x_to_source_x(1000);

        assert!((converted_left-(-1.0 * 1.0 + 2.0)).abs() < 0.01, "Expected {}, got {}", (-1.0 * 1.0 + 2.0), converted_left);
        assert!((converted_right-(1.0 * 1.0 + 2.0)).abs() < 0.01, "Expected {}, got {}", (1.0 * 1.0 + 2.0), converted_right);
    }

    #[test]
    fn transform_translate_2() {
        let transformed     = ScanlineTransform::for_region(&(-1.0..1.0), 1000).transform(1.0, 2.0);
        let converted_left  = transformed.source_x_to_pixels(-1.0 * 1.0 + 2.0);
        let converted_right = transformed.source_x_to_pixels(1.0 * 1.0 + 2.0);

        assert!((converted_left-(0.0)).abs() < 0.01, "Expected {}, got {}", 0.0, converted_left);
        assert!((converted_right-(1000.0)).abs() < 0.01, "Expected {}, got {}", 1000.0, converted_right);
    }

    #[test]
    fn transform_scale_translate_1() {
        let transformed     = ScanlineTransform::for_region(&(-1.0..1.0), 1000).transform(2.0, 3.0);
        let converted_left  = transformed.pixel_x_to_source_x(0);
        let converted_right = transformed.pixel_x_to_source_x(1000);

        assert!((converted_left-(-1.0 * 2.0 + 3.0)).abs() < 0.01, "Expected {}, got {}", (-1.0 * 2.0 + 3.0), converted_left);
        assert!((converted_right-(1.0 * 2.0 + 3.0)).abs() < 0.01, "Expected {}, got {}", (1.0 * 2.0 + 3.0), converted_right);
    }

    #[test]
    fn transform_scale_translate_2() {
        let transformed     = ScanlineTransform::for_region(&(-1.0..1.0), 1000).transform(2.0, 3.0);
        let converted_left  = transformed.source_x_to_pixels(-1.0 * 2.0 + 3.0);
        let converted_right = transformed.source_x_to_pixels(1.0 * 2.0 + 3.0);

        assert!((converted_left-(0.0)).abs() < 0.01, "Expected {}, got {}", 0.0, converted_left);
        assert!((converted_right-(1000.0)).abs() < 0.01, "Expected {}, got {}", 1000.0, converted_right);
    }
}
