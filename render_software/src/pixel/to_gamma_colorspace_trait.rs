///
/// Trait implemented by pixel types that can be converted to a gamma-corrected colour space
///
pub trait ToGammaColorSpace<TargetPixel> {
    /// Converts this pixel from its current colour space to a gamma corrected colour space
    fn to_gamma_colorspace(&self, gamma: f64) -> TargetPixel;
}
