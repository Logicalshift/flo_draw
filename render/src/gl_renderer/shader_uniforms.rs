///
/// The uniforms for shaders used by the 2D rendering engine
///
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub enum ShaderUniform {
    /// The transformation matrix to use
    Transform,
    
    /// The texture bound to the 'erase' operation
    EraseTexture,

    /// The texture bound to the 'clip' operation
    ClipTexture,

    /// The texture used for the dash pattern
    DashTexture,

    /// Texture used for picking the colour of a fragment
    Texture,

    /// The transform applied to the texture coordinates
    TextureTransform
}
