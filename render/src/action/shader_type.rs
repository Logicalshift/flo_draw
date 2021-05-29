use super::identities::*;

use crate::buffer::*;

///
/// The shaders that can be chosen for the renderer
///
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ShaderType {
    /// Flat colour shader
    /// The erase texture (which should be a MSAA texture) is subtracted from anything drawn, if present
    Simple { erase_texture: Option<TextureId>, clip_texture: Option<TextureId> },

    /// Flat colour with 'dashed line' texturing using a 1D texture
    DashedLine { dash_texture: TextureId, erase_texture: Option<TextureId>, clip_texture: Option<TextureId> },

    /// Colour derived from a texture with a transform mapping from canvas coordinates to texture coordinates
    Texture { texture: TextureId, texture_transform: Matrix, repeat: bool, alpha: f32, erase_texture: Option<TextureId>, clip_texture: Option<TextureId> },

    /// Colour dervide from a 1D texture using a transform mapping (used for rendering linear gradients)
    LinearGradient { texture: TextureId, texture_transform: Matrix, repeat: bool, alpha: f32, erase_texture: Option<TextureId>, clip_texture: Option<TextureId> }
}

impl ShaderType {
    ///
    /// Adds an erase mask texture to the existing shader
    ///
    pub fn with_erase_mask(self, new_erase_mask_texture: Option<TextureId>) -> ShaderType {
        use self::ShaderType::*;

        match self {
            Simple { erase_texture: _, clip_texture }                                                       => Simple           { erase_texture: new_erase_mask_texture, clip_texture: clip_texture },
            DashedLine { dash_texture, erase_texture: _, clip_texture }                                     => DashedLine       { dash_texture: dash_texture, erase_texture: new_erase_mask_texture, clip_texture: clip_texture },
            Texture { texture, texture_transform, repeat, alpha, erase_texture: _, clip_texture }           => Texture          { texture: texture, texture_transform: texture_transform, repeat, alpha, erase_texture: new_erase_mask_texture, clip_texture: clip_texture },
            LinearGradient { texture, texture_transform, repeat, alpha, erase_texture: _, clip_texture }    => LinearGradient   { texture: texture, texture_transform: texture_transform, repeat, alpha, erase_texture: new_erase_mask_texture, clip_texture: clip_texture }
        }
    }

    ///
    /// Adds a clip mask texture to the existing shader
    ///
    pub fn with_clip_mask(self, new_clip_mask_texture: Option<TextureId>) -> ShaderType {
        use self::ShaderType::*;

        match self {
            Simple { erase_texture, clip_texture: _ }                                                       => Simple           { erase_texture: erase_texture, clip_texture: new_clip_mask_texture },
            DashedLine { dash_texture, erase_texture, clip_texture: _ }                                     => DashedLine       { dash_texture: dash_texture, erase_texture: erase_texture, clip_texture: new_clip_mask_texture },
            Texture { texture, texture_transform, repeat, alpha, erase_texture, clip_texture: _ }           => Texture          { texture: texture, texture_transform: texture_transform, repeat, alpha, erase_texture: erase_texture, clip_texture: new_clip_mask_texture },
            LinearGradient { texture, texture_transform, repeat, alpha, erase_texture, clip_texture: _ }    => LinearGradient   { texture: texture, texture_transform: texture_transform, repeat, alpha, erase_texture: erase_texture, clip_texture: new_clip_mask_texture }
        }
    }
}
