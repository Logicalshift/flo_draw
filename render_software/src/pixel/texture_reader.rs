///
/// Trait implemented by types that can be read from a texture type
///
pub trait TextureReader<TTexture> : Send + Sync + Sized
where
    TTexture: Send + Sync,
{
    ///
    /// Reads a sequence of pixels from this texture into a target array
    ///
    /// Coordinates are in fractions of pixels to allow for a texture reader to support bilinear interpolation or mipmapping
    ///
    fn read_pixels(texture: &TTexture, pixels: &mut [Self], positions: &[(f64, f64)]);
}
