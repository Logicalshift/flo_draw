use super::shader_cache::*;

use wgpu;

use std::sync::*;
use std::borrow::{Cow};

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
/// How the texture points are determined by the shader
///
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum TexturePosition {
    /// Input position multiplied by texture transform
    InputPosition,

    /// Stored in the tex_coord parameter in the vertex buffer
    Separate,
}

///
/// The type of texture used as input for a texture shader
///
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputTextureType {
    /// Using no texture
    None,

    /// Using a texture sampler
    Sampler,

    /// Using a multi-sampled texture that needs to be resolved
    Multisampled,
}

///
/// Enumeration of the shaders loaded for the WGPU renderer
///
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum WgpuShader {
    /// Flat colour shader
    Simple(StandardShaderVariant, ColorPostProcessingStep),

    /// Renders fragments from a texture input
    Texture(StandardShaderVariant, InputTextureType, TexturePosition, AlphaBlendStep, ColorPostProcessingStep),
}

impl Default for WgpuShader {
    fn default() -> WgpuShader {
        WgpuShader::Simple(StandardShaderVariant::NoClipping, ColorPostProcessingStep::NoPostProcessing)
    }
}

impl ColorPostProcessingStep {
    ///
    /// Retrieves the `color_post_process` function for this post-processing step
    ///
    fn shader_function(&self) -> &'static str {
        match self {
            ColorPostProcessingStep::NoPostProcessing   => include_str!("../../shaders/simple/color_no_post_processing.wgsl"),
            ColorPostProcessingStep::MultiplyAlpha      => include_str!("../../shaders/simple/color_multiply_alpha.wgsl"),
            ColorPostProcessingStep::InvertColorAlpha   => include_str!("../../shaders/simple/color_invert_alpha.wgsl"),
        }
    }
}

impl StandardShaderVariant {
    fn shader_function(&self) -> &'static str {
        match self {
            StandardShaderVariant::NoClipping   => include_str!("../../shaders/simple/clip_none.wgsl"),
            StandardShaderVariant::ClippingMask => include_str!("../../shaders/simple/clip_mask.wgsl"),
        }
    }
}

impl AlphaBlendStep {
    fn shader_function(&self) -> &'static str {
        match self {
            AlphaBlendStep::NoPremultiply   => include_str!("../../shaders/texture/alpha_no_premultiply.wgsl"),
            AlphaBlendStep::Premultiply     => include_str!("../../shaders/texture/alpha_premultiplied.wgsl"),
        }
    }
}

impl InputTextureType {
    fn shader_function(&self) -> &'static str {
        match self {
            InputTextureType::None          => include_str!("../../shaders/texture/texture_none.wgsl"),
            InputTextureType::Sampler       => include_str!("../../shaders/texture/texture_sampler.wgsl"),
            InputTextureType::Multisampled  => include_str!("../../shaders/texture/texture_multisample.wgsl"),
        }
    }
}

impl TexturePosition {
    fn shader_function(&self) -> &'static str {
        match self {
            TexturePosition::InputPosition  => include_str!("../../shaders/texture/texture_pos_input.wgsl"),
            TexturePosition::Separate       => include_str!("../../shaders/texture/texture_pos_separate.wgsl"),
        }
    }
}


impl WgpuShaderLoader for WgpuShader {
    ///
    /// Loads the appropriate shader, and returns the entry point to use for the fragment and vertex shaders
    ///
    fn load(&self, device: &wgpu::Device) -> (Arc<wgpu::ShaderModule>, String, String) {
        match self {
            WgpuShader::Simple(variant, color_post_processing) => {
                // The base module contains the shader program in terms of the variant and post-procesing functions
                let base_module = include_str!("../../shaders/simple/simple.wgsl");

                // Amend the base module with the appropriate variant and colour post-processing functions
                let base_module = format!("{}\n\n{}\n\n{}", variant.shader_function(), color_post_processing.shader_function(), base_module);

                // Load the shader
                let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label:  Some("WgpuShader::Simple"),
                    source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&base_module)),
                });

                (Arc::new(shader_module), "simple_vertex_shader".to_string(), "simple_fragment_shader".to_string())
            },

            WgpuShader::Texture(variant, input_type, texture_position, alpha_blend, color_post_processing) => {
                // The base module contains the shader program in terms of the variant and post-procesing functions
                let base_module = include_str!("../../shaders/texture/texture.wgsl");

                // Amend the base module with the appropriate variant and colour post-processing functions
                let base_module = format!("{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}", 
                    variant.shader_function(), 
                    texture_position.shader_function(), 
                    alpha_blend.shader_function(), 
                    input_type.shader_function(), 
                    color_post_processing.shader_function(), base_module);

                // Load the shader
                let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label:  Some("WgpuShader::Texture"),
                    source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&base_module)),
                });

                (Arc::new(shader_module), "texture_vertex_shader".to_string(), "texture_fragment_shader".to_string())
            },
        }
    }
}
