use super::shader_cache::*;
use super::render_pass_resources::*;
use super::pipeline_configuration::*;
use crate::buffer::*;

use wgpu;
use wgpu::util;
use wgpu::util::{DeviceExt};

use std::mem;
use std::slice;
use std::sync::*;
use std::ffi::{c_void};

///
/// State for the WGPU renderer
///
pub (crate) struct RendererState {
    /// The command queue for the device
    queue:                              Arc<wgpu::Queue>,

    /// The command encoder for this rendering
    encoder:                            wgpu::CommandEncoder,

    /// The resources for the next render pass
    pub render_pass_resources:          RenderPassResources,

    /// The pipeline configuration to use with the current rendering
    pub pipeline_configuration:         PipelineConfiguration,

    /// Set to true if the pipeline configuration has changed since it was last committed to the render pass
    pub pipeline_config_changed:        bool,

    /// The pipeline configuration that was last activated
    pub active_pipeline_configuration:  Option<PipelineConfiguration>,

    /// The actions for the active render pass (deferred so we can manage the render pass lifetime)
    pub render_pass:                    Vec<Box<dyn for<'a> FnOnce(&'a RenderPassResources, &mut wgpu::RenderPass<'a>) -> ()>>,

    /// The matrix transform buffer
    matrix_buffer:                      wgpu::Buffer,

    /// The binding group for the matrix buffer
    matrix_binding:                     wgpu::BindGroup,
}

impl RendererState {
    ///
    /// Creates a default render state
    ///
    pub fn new(command_queue: Arc<wgpu::Queue>, device: &wgpu::Device) -> RendererState {
        // TODO: we can avoid re-creating some of these structures every frame: eg, the binding groups in particular

        // Create all the state structures
        let (matrix_buffer, matrix_binding) = Self::create_transform_buffer(&device);
        let encoder                         = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("RendererState::new") });

        RendererState {
            queue:                              command_queue,
            encoder:                            encoder,
            render_pass_resources:              RenderPassResources::default(),
            render_pass:                        vec![],
            pipeline_configuration:             PipelineConfiguration::default(),
            pipeline_config_changed:            true,
            active_pipeline_configuration:      None,

            matrix_buffer:                      matrix_buffer,
            matrix_binding:                     matrix_binding,
        }
    }

    ///
    /// Updates the contents of the matrix buffer for this renderer
    ///
    #[inline]
    pub fn write_matrix(&self, new_matrix: &Matrix) {
        let matrix_void     = new_matrix.0.as_ptr() as *const c_void;
        let matrix_len      = mem::size_of::<[[f32; 4]; 4]>();
        let matrix_u8       = unsafe { slice::from_raw_parts(matrix_void as *const u8, matrix_len) };

        self.queue.write_buffer(&self.matrix_buffer, 0, matrix_u8);
    }

    ///
    /// Sets up the transform buffer and layout
    ///
    fn create_transform_buffer(device: &wgpu::Device) -> (wgpu::Buffer, wgpu::BindGroup) {
        // Convert the matrix to a u8 pointer
        let matrix          = Matrix::identity();
        let matrix_void     = matrix.0.as_ptr() as *const c_void;
        let matrix_len      = mem::size_of::<[[f32; 4]; 4]>();
        let matrix_u8       = unsafe { slice::from_raw_parts(matrix_void as *const u8, matrix_len) };

        // Load into a buffer
        let matrix_buffer   = device.create_buffer_init(&util::BufferInitDescriptor {
            label:      Some("matrix_buffer"),
            contents:   matrix_u8,
            usage:      wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create a binding group
        let matrix_layout   = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label:      Some("matrix_layout"),
            entries:    &[
                wgpu::BindGroupLayoutEntry {
                    binding:    0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    count:      None,
                    ty:         wgpu::BindingType::Buffer {
                        ty:                 wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size:   None,
                    }    
                }
            ],
        });

        let matrix_binding  = device.create_bind_group(&wgpu::BindGroupDescriptor {
             label:     Some("matrix_binding"),
             layout:    &matrix_layout,
             entries:   &[
                wgpu::BindGroupEntry {
                    binding:    0,
                    resource:   matrix_buffer.as_entire_binding(),
                }
             ]
        });

        (matrix_buffer, matrix_binding)
    }

    ///
    /// Runs the pending render pass
    ///
    pub fn run_render_pass(&mut self) {
        // Take the actions and the resources for this render pass
        let render_actions  = mem::take(&mut self.render_pass);
        let resources       = mem::take(&mut self.render_pass_resources);

        // Keep the current texture view for the next render pass
        self.render_pass_resources.target_view  = resources.target_view.clone();

        // This resets the active pipeline configuration
        self.active_pipeline_configuration      = None;
        self.pipeline_config_changed            = true;

        // Abort early if there are no render actions
        if render_actions.is_empty() {
            return;
        }

        // Start a new render pass using the encoder
        if let Some(texture_view) = &resources.target_view {
            // Start the render pass
            let mut render_pass = self.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label:                      Some("run_render_pass"),
                depth_stencil_attachment:   None,
                color_attachments:          &resources.color_attachments(),
            });

            // Run all of the actions
            for action in render_actions.into_iter() {
                (action)(&resources, &mut render_pass);
            }
        }
    }
}