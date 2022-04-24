///
/// The uniforms for shaders used by the 2D rendering engine
///
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub enum ShaderUniform {
    /// The transformation matrix to use
    Transform,

    /// The texture bound to the 'clip' operation
    ClipTexture,

    /// The texture used for the dash pattern
    DashTexture,

    /// Texture used for picking the colour of a fragment
    Texture,

    /// The transform applied to the texture coordinates
    TextureTransform,

    /// The alpha adjustment applied to the texture colour
    TextureAlpha,

    /// The texture for a MSAA shader
    MsaaTexture,

    /// The alpha value to use for a MSAA shader
    MsaaAlpha,

    /// The weights for the gaussian blur shader
    BlurWeights,

    /// The offsets for the gaussian blur shader
    BlurOffsets,

    /// The weights for the gaussian blur shader (when defined as a texture)
    TextureBlurWeights,

    /// The weights for the gaussian blur shader (when defined as a texture)
    TextureBlurOffsets,

    /// The input texture for a filter that needs one
    FilterTexture,

    /// The scale factor used for a filter
    FilterScale,
}
