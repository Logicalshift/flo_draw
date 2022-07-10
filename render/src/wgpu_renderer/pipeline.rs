use super::wgpu_shader::*;
use super::shader_cache::*;
use super::pipeline_configuration::*;

use wgpu;

use std::sync::*;

///
/// A render pipeline and its binding groups
///
pub (crate) struct Pipeline {
    /// The shader module for this pipeline
    pub (crate) shader_module: WgpuShader,

    /// The render pipeline
    pub (crate) pipeline: Arc<wgpu::RenderPipeline>,

    /// The bind group layout for the transformation matrix
    pub (crate) matrix_layout: Arc<wgpu::BindGroupLayout>,

    /// The bind group layout for the clip mask
    pub (crate) clip_mask_layout: Arc<wgpu::BindGroupLayout>,
}

impl Pipeline {
    ///
    /// Creates a pipeline from a pipline configuration
    ///
    pub fn from_configuration(config: &PipelineConfiguration, device: &wgpu::Device, shader_cache: &mut ShaderCache<WgpuShader>) -> Pipeline {
        let mut temp_data       = PipelineDescriptorTempStorage::default();
        
        let matrix_bind_layout  = config.matrix_bind_group_layout();
        let clip_bind_layout    = config.clip_mask_bind_group_layout();
        let matrix_bind_layout  = device.create_bind_group_layout(&matrix_bind_layout);
        let clip_bind_layout    = device.create_bind_group_layout(&clip_bind_layout);

        let bind_layout         = [&matrix_bind_layout, &clip_bind_layout];
        let pipeline_layout     = wgpu::PipelineLayoutDescriptor {
            label:                  Some("Pipeline::from_configuration"),
            bind_group_layouts:     &bind_layout,
            push_constant_ranges:   &[],
        };
        let pipeline_layout     = device.create_pipeline_layout(&pipeline_layout);

        let descriptor          = config.render_pipeline_descriptor(shader_cache, &pipeline_layout, &mut temp_data);
        let new_pipeline        = device.create_render_pipeline(&descriptor);

        Pipeline {
            shader_module:      config.shader_module.clone(),
            pipeline:           Arc::new(new_pipeline),
            matrix_layout:      Arc::new(matrix_bind_layout),
            clip_mask_layout:   Arc::new(clip_bind_layout),
        }
    }

    ///
    /// Returns the index of the matrix binding group
    ///
    #[inline]
    pub fn matrix_group_index(&self) -> u32 {
        0
    }

    ///
    /// Returns the index of the clip mask binding group
    ///
    #[inline]
    pub fn clip_mask_group_index(&self) -> u32 {
        1
    }

    ///
    /// Binds the transformation matrix buffer for this pipeline (filling in or replacing the `matrix_binding` entry)
    ///
    #[inline]
    pub fn bind_matrix_buffer(&self, device: &wgpu::Device, matrix_buffer: &wgpu::Buffer) -> wgpu::BindGroup {
        let matrix_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label:      Some("bind_matrix"),
            layout:     &*self.matrix_layout,
            entries:    &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: matrix_buffer.as_entire_binding(),
                }
            ]
        });

        matrix_bind_group
    }

    ///
    /// Creates the clip mask binding group for this pipeline configuration
    ///
    /// This is stored in bind group 1. The clip texture must be supplied for a valid bind group to be generated if the shader is using the clipping mask
    /// (it's optional because it's not otherwise required)
    ///
    pub fn bind_clip_mask(&self, device: &wgpu::Device, clip_texture: Option<&wgpu::Texture>) -> wgpu::BindGroup {
        match (&self.shader_module, clip_texture) {
            (WgpuShader::Texture(StandardShaderVariant::ClippingMask, _, _, _), Some(clip_texture)) |
            (WgpuShader::Simple(StandardShaderVariant::ClippingMask, _), Some(clip_texture))        => {
                // Create a view of the texture
                let view = clip_texture.create_view(&wgpu::TextureViewDescriptor::default());

                // Bind to group 1
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label:      Some("create_clip_mask_bind_group_with_texture"),
                    layout:     &*self.clip_mask_layout,
                    entries:    &[
                        wgpu::BindGroupEntry {
                            binding:    0,
                            resource:   wgpu::BindingResource::TextureView(&view),
                        }
                    ]
                })
            }

            (_, None)                                                               |
            (WgpuShader::Texture(StandardShaderVariant::NoClipping, _, _, _), _)    |
            (WgpuShader::Simple(StandardShaderVariant::NoClipping, _), _)           => {
                // Group 1 is bound to an empty set if clipping is off or no texture is defined
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label:      Some("create_clip_mask_bind_group_no_texture"),
                    layout:     &*self.clip_mask_layout,
                    entries:    &[]
                })
            }
        }
    }
}
