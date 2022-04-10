///
/// Filters that can be applied to a texture by the rendering engine
///
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum TextureFilter {
    /// Applies a horizontal gaussian blur with the specified sigma (standard deviation) value, using a 5-pixel kernel
    GaussianBlurHorizontal5(f32),

    /// Applies a vertical gaussian blur with the specified sigma (standard deviation) value, using a 5-pixel kernel
    GaussianBlurVertical5(f32),
}
