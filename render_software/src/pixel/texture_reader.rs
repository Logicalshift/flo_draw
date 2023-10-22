///
/// Trait implemented by types that can be read from a texture type
///
pub trait TextureReader<TTexture> : Send + Sync 
where
    TTexture: Send + Sync,
{
    /// Reads the pixel at the specified position in the texture
    ///
    /// The coordinates are fractions of pixels
    fn read_pixel(texture: &TTexture, x: f64, y: f64) -> Self;
}
