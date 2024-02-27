///
/// A mip-map is a texture scaled down to half-sizes
///
/// This can be used to estimate the colour of a pixel when scaling a texture down
///
#[derive(Clone)]
pub struct MipMap<TTexture> {
    /// The width of the texture
    width: usize,

    /// The height of the texture
    height: usize,

    /// The texture stored in this mipmap
    mip_levels: Vec<TTexture>,
}

impl<TTexture> MipMap<TTexture> {
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

        MipMap { width, height, mip_levels }
    }

    ///
    /// The width of the top-level mipmap
    ///
    #[inline]
    pub fn width(&self) -> usize { self.width }

    ///
    /// The height of the top-level mipmap
    ///
    #[inline]
    pub fn height(&self) -> usize { self.height }

    ///
    /// Returns the mip level to use for reading a texture where the samples are taken at a particular offset (measured in pixels)
    ///
    #[inline]
    pub fn level_for_pixel_step(&self, dx: f64, dy: f64) -> usize {
        // Get the number of pixels covered by each step
        let pixel_step          = (dx*dx + dy*dy).sqrt();
        let approx_pixel_step   = pixel_step.floor() as usize;

        if approx_pixel_step == 0 {
            0
        } else {
            // The mip level is the log2 of the pixel step
            let level = approx_pixel_step.ilog2() as usize;

            level.min(self.mip_levels.len()-1)
        }
    }

    ///
    /// Retrieves the texture for a particular mip level
    ///
    #[inline]
    pub fn mip_level(&self, level: usize) -> &TTexture {
        &self.mip_levels[level]
    }
}
