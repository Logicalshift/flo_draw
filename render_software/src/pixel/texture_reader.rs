///
/// Trait implemented by types that can be read from a texture type
///
pub trait TextureReader<TTexture> : Send + Sync + Sized
where
    TTexture: Send + Sync,
{
    ///
    /// Returns the size of a texture in pixels
    ///
    fn texture_size(texture: &TTexture) -> (f64, f64);

    ///
    /// Reads a sequence of pixels from this texture into a target array
    ///
    /// Coordinates are in fractions of pixels to allow for a texture reader to support bilinear interpolation or mipmapping
    ///
    fn read_pixels(texture: &TTexture, positions: &[(f64, f64)]) -> Vec<Self>;

    ///
    /// Reads a set of pixels across a linear gradient
    ///
    /// This is a common way that pixels are read out from a texture, so this function can be overridden to optimise this
    /// for different types of texture storage if needed.
    ///
    /// This reads `count` pixels at locations `t = 0, 1, 2, ...` such that `u = dx * t + offset` and `x = x_gradient.0 * u + x_gradient.1`,
    /// `y = y_gradient.0 * u + y_gradient.1`.
    ///
    #[inline]
    fn read_pixels_linear(texture: &TTexture, offset:f64, dx: f64, x_gradient: (f64, f64), y_gradient: (f64, f64), count: usize) -> Vec<Self> {
        // Allocate enough space to store the pixels
        let mut positions = Vec::with_capacity(count);

        // Calculate the positions for the pixels
        positions.extend((0..count).map(|t| {
            let t = t as f64;
            let u = dx * t + offset;
            let x = x_gradient.0 * u + x_gradient.1;
            let y = y_gradient.0 * u + y_gradient.1;

            (x, y)
        }));

        Self::read_pixels(texture, &positions)
    }
}
