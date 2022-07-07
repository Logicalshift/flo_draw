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
            label:                  Some("Pipeline::from_configuration"),
            bind_group_layouts:     &bind_layout,
            push_constant_ranges:   &[],
        };
        let pipeline_layout     = device.create_pipeline_layout(&pipeline_layout);

        let descriptor          = config.render_pipeline_descriptor(shader_cache, &pipeline_layout, &mut temp_data);
        let new_pipeline        = device.create_render_pipeline(&descriptor);

        Pipeline {
            pipeline:           Arc::new(new_pipeline),
            matrix_layout:      Some(Arc::new(matrix_bind_layout)),
        }
    }

    ///
    /// Returns the index of the matrix binding group
    ///
    #[inline]
    pub fn matrix_group_index(&self) -> Option<u32> {
        if let Some(matrix_layout) = &self.matrix_layout {
            Some(0)
        } else {
            None
        }
    }

    ///
    /// Binds the transformation matrix buffer for this pipeline (filling in or replacing the `matrix_binding` entry)
    ///
    #[inline]
    pub fn bind_matrix_buffer(&self, device: &wgpu::Device, matrix_buffer: &wgpu::Buffer) -> Option<wgpu::BindGroup> {
        if let Some(matrix_layout) = &self.matrix_layout {
            let matrix_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label:      Some("bind_matrix"),
                layout:     &matrix_layout,
                entries:    &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: matrix_buffer.as_entire_binding(),
                    }
                ]
            });

            Some(matrix_bind_group)
        } else {
            None
        }
    }
}
