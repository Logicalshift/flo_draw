use super::identities::*;

///
/// The shaders that can be chosen for the renderer
///
#[derive(Clone, Copy, PartialEq, Debug, Hash)]
pub enum ShaderType {
    /// Flat colour shader
    /// The erase texture (which should be a MSAA texture) is subtracted from anything drawn, if present
    Simple { erase_texture: Option<TextureId>, clip_texture: Option<TextureId> },
}
