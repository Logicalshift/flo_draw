///
/// A mip-map is a texture scaled down to half-sizes
///
/// This can be used to estimate the colour of a pixel when scaling a texture down
///
#[derive(Clone)]
pub struct MipMapTexture<TTexture> {
    /// The width of the texture
    width: usize,

    /// The height of the texture
    height: usize,

    /// The texture stored in this mipmap
    mip_levels: Vec<TTexture>,
}

impl<TTexture> MipMapTexture<TTexture> {
    ///
    /// Creates a new mip-map by calculating the levels from a texture
    ///
    /// The width and height is the size of the first texture. Each level should have half the width and height of the previous level,
    /// and the function should return 'None' if no further levels can be generated. Halving the size of a texture like this should make
    /// the maximum memory requirements of the resulting mip-map only about twice as much as the original texture.
    ///
    pub fn from_texture(texture: TTexture, create_next_mip_level: impl Fn(&TTexture) -> Option<TTexture>, width: usize, height: usize) -> Self {
        // Create the mip levels
        let mut mip_levels  = vec![];
        let mut last_level  = texture;

        while let Some(next_level) = create_next_mip_level(&last_level) {
            mip_levels.push(last_level);
            last_level = next_level;
        }

        // last_level will contain the final level of the mipmap
        mip_levels.push(last_level);

        MipMapTexture { width, height, mip_levels }
    }
}
