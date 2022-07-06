use super::wgpu_shader::*;
use super::shader_cache::*;
use super::pipeline_configuration::*;

use wgpu;

use std::sync::*;

///
/// A render pipeline and its binding groups
///
pub (crate) struct Pipeline {
    /// The render pipeline
    pub (crate) pipeline: Arc<wgpu::RenderPipeline>,

    /// The bind group layout for the transformation matrix
    pub (crate) matrix_layout: Option<Arc<wgpu::BindGroupLayout>>,

    /// The bind group added to the render pipeline for the matrix binding
    pub (crate) matrix_binding: Option<Arc<wgpu::BindGroup>>,
}

impl Pipeline {
    ///
    /// Creates a pipeline from a pipline configuration
    ///
    pub fn from_configuration(config: &PipelineConfiguration, device: &wgpu::Device, shader_cache: &mut ShaderCache<WgpuShader>) -> Pipeline {
        let mut temp_data       = PipelineDescriptorTempStorage::default();
        
        let matrix_bind_layout  = config.matrix_bind_group_layout();
        let matrix_bind_layout  = device.create_bind_group_layout(&matrix_bind_layout);

        let bind_layout         = [&matrix_bind_layout];
        let pipeline_layout     = wgpu::PipelineLayoutDescriptor {
            label:                  Some("pipeline_layout"),
            bind_group_layouts:     &bind_layout,
            push_constant_ranges:   &[],
        };
        let pipeline_layout     = device.create_pipeline_layout(&pipeline_layout);

        let descriptor          = config.render_pipeline_descriptor(shader_cache, &pipeline_layout, &mut temp_data);
        let new_pipeline        = device.create_render_pipeline(&descriptor);

        Pipeline {
            pipeline:           Arc::new(new_pipeline),
            matrix_layout:      Some(Arc::new(matrix_bind_layout)),
            matrix_binding:     None,
        }
    }
}