use super::wgpu_shader::*;
use crate::action::*;

use wgpu;

///
/// Description of a WGPU pipeline configuration (used to create the configuration and as a hash key)
///
#[derive(Clone, PartialEq, Hash)]
pub (crate) struct PipelineConfiguration {
    /// Format of the texture that this will render against
    pub (crate) texture_format: wgpu::TextureFormat,

    /// The identifier of the shader module to use (this defines both the vertex and the fragment shader, as well as the pipeline layout to use)
    pub (crate) shader_module: WgpuShader,

    /// The blending mode for this pipeline configuration
    pub (crate) blending_mode: BlendMode,
}
