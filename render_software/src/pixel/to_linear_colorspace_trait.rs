///
/// Trait implemented by pixel types that can convert themselves to a linear colour space
///
/// Linear colour spaces have no gamma correction applied to their values (ie, doubling the size of a component
/// doubles its brightness)
///
pub trait ToLinearColorSpace<TargetPixel> : Sized {
    ///
    /// Converts a set of pixels to the target pixel type
    ///
    fn to_linear_colorspace(input_pixels: &[Self], output_pixels: &mut [TargetPixel]);
}
