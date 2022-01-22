use super::identities::*;

use crate::buffer::*;

///
/// The shaders that can be chosen for the renderer
///
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ShaderType {
    /// Flat colour shader
    /// The erase texture (which should be a MSAA texture) is subtracted from anything drawn, if present
    Simple { clip_texture: Option<TextureId> },

    /// Flat colour with 'dashed line' texturing using a 1D texture
    DashedLine { dash_texture: TextureId, clip_texture: Option<TextureId> },

    /// Colour derived from a texture with a transform mapping from canvas coordinates to texture coordinates
    Texture { texture: TextureId, texture_transform: Matrix, repeat: bool, alpha: f32, clip_texture: Option<TextureId> },

    /// Colour derived from a 1D texture using a transform mapping (used for rendering linear gradients)
    LinearGradient { texture: TextureId, texture_transform: Matrix, repeat: bool, alpha: f32, clip_texture: Option<TextureId> }
}

impl ShaderType {
    ///
    /// Adds a clip mask texture to the existing shader
    ///
    pub fn with_clip_mask(self, new_clip_mask_texture: Option<TextureId>) -> ShaderType {
        use self::ShaderType::*;

        match self {
            Simple { clip_texture: _ }                                                      => Simple           { clip_texture: new_clip_mask_texture },
            DashedLine { dash_texture, clip_texture: _ }                                    => DashedLine       { dash_texture: dash_texture, clip_texture: new_clip_mask_texture },
            Texture { texture, texture_transform, repeat, alpha, clip_texture: _ }          => Texture          { texture: texture, texture_transform: texture_transform, repeat, alpha, clip_texture: new_clip_mask_texture },
            LinearGradient { texture, texture_transform, repeat, alpha, clip_texture: _ }   => LinearGradient   { texture: texture, texture_transform: texture_transform, repeat, alpha, clip_texture: new_clip_mask_texture }
        }
    }
}
