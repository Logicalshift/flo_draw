///
/// Trait implemented by types that can be read from a texture type
///
pub trait TextureReader<TTexture> : Send + Sync + Sized
where
    TTexture: Send + Sync,
{
    /// Reads the pixel at the specified position in the texture
    ///
    /// The coordinates are fractions of pixels
    fn read_pixel(texture: &TTexture, x: f64, y: f64) -> Self;

    ///
    /// Reads a sequence of pixels from this texture into a target array
    ///
    #[inline]
    fn read_pixels(texture: &TTexture, pixels: &mut [Self], positions: &[(f64, f64)]) {
        for ((x, y), pixel) in positions.iter().zip(pixels.iter_mut()) {
            *pixel = Self::read_pixel(texture, *x, *y);
        }
    }
}
