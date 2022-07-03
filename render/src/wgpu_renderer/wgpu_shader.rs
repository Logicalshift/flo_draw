///
/// The variants that every shader must have
///
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StandardShaderVariant {
    /// No clipping texture is applied to the shader
    NoClipping,

    /// A clipping texture is used to mask the rendering
    ClippingMask
}

///
/// The post-processing step to apply to the colour output of a shader
///
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColorPostProcessingStep {
    /// The shader should not change the colour outputs
    NoPostProcessing,

    /// The shader should multiply its outputs by the alpha value
    MultiplyAlpha,

    /// The colour is blended so that at alpha (0), the RGB values are (1,1,1) - the inverse of pre-multiplications
    InvertColorAlpha,
}

///
/// Describes what to do when applying an alpha value to a pixel
///
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AlphaBlendStep {
    /// Input colours are not pre-multiplied
    NoPremultiply,

    /// Input colours are pre-mulitplied
    Premultiply,
}

///
/// The format of the source texture for a filter step
///
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FilterSourceFormat {
    /// Alpha is pre-multiplied
    PremultipliedAlpha,

    /// Alpha is not pre-multiplied
    NotPremultiplied,
}

///
/// Enumeration of the shaders loaded for the WGPU renderer
///
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum WgpuShader {
    /// Flat colour shader
    Simple(StandardShaderVariant, ColorPostProcessingStep),
}

impl Default for WgpuShader {
    fn default() -> WgpuShader {
        WgpuShader::Simple(StandardShaderVariant::NoClipping, ColorPostProcessingStep::NoPostProcessing)
    }
}
