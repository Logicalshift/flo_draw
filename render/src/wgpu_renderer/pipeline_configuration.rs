use super::render_target::*;

use wgpu;

///
/// Description of a WGPU pipeline configuration (used to create the configuration and as a hash key)
///
#[derive(Clone, PartialEq, Hash)]
pub (crate) struct PipelineConfiguration {
    pub (crate) texture_format: wgpu::TextureFormat,
}

impl PipelineConfiguration {
    ///
    /// Creates a render pipeline descriptor from this configuration
    ///
    pub fn to_render_pipeline_description(&self, adapter: &wgpu::Adapter, surface: &wgpu::Surface) -> wgpu::RenderPipelineDescriptor {
        let fragment_format = wgpu::ColorTargetState::from(self.texture_format);

        unimplemented!()
    }
}