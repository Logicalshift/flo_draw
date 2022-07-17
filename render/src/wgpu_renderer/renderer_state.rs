use super::pipeline::*;
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
    /// The device this will render to
    device:                             Arc<wgpu::Device>,

    /// The command queue for the device
    queue:                              Arc<wgpu::Queue>,

    /// The command encoder for this rendering
    pub encoder:                        wgpu::CommandEncoder,

    /// The resources for the next render pass
    pub render_pass_resources:          RenderPassResources,

    /// The pipeline configuration to use with the current rendering
    pub pipeline_configuration:         PipelineConfiguration,

    /// The active pipeline
    pub pipeline:                       Option<Arc<Pipeline>>,

    /// Set to true if the pipeline configuration has changed since it was last committed to the render pass
    pub pipeline_config_changed:        bool,

    /// True if the pipeline bindings have been updated since they were last written
    pub pipeline_bindings_changed:      bool,

    /// The pipeline configuration that was last activated
    pub active_pipeline_configuration:  Option<PipelineConfiguration>,

    /// The actions for the active render pass (deferred so we can manage the render pass lifetime)
    pub render_pass:                    Vec<Box<dyn for<'a> FnOnce(&'a RenderPassResources, &mut wgpu::RenderPass<'a>) -> ()>>,

    /// The size of the current render target
    pub target_size:                    (u32, u32),

    /// The last transform matrix set
    pub active_matrix:                  Matrix,

    /// The texture transform buffer
    pub texture_transform:              Arc<wgpu::Buffer>,

    /// The input texture set for the current shader (or none)
    pub input_texture:                  Option<Arc<wgpu::Texture>>,

    /// The texture used for clipping the image
    pub clip_texture:                   Option<Arc<wgpu::Texture>>,

    /// The sampler for the current shader (or none)
    pub sampler:                        Option<Arc<wgpu::Sampler>>,

    /// The buffer containing the alpha value for the current texture
    pub texture_alpha:                  Option<Arc<wgpu::Buffer>>,
}

impl RendererState {
    ///
    /// Creates a default render state
    ///
    pub fn new(command_queue: Arc<wgpu::Queue>, device: Arc<wgpu::Device>) -> RendererState {
        // TODO: we can avoid re-creating some of these structures every frame: eg, the binding groups in particular

        // Create all the state structures
        let texture_transform   = Arc::new(Self::create_transform_buffer(&device, &Matrix::identity()));
        let encoder             = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("RendererState::new") });

        let default_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("default_sampler"),
            address_mode_u:     wgpu::AddressMode::Repeat,
            address_mode_v:     wgpu::AddressMode::Repeat,
            address_mode_w:     wgpu::AddressMode::Repeat,
            mag_filter:         wgpu::FilterMode::Nearest,
            min_filter:         wgpu::FilterMode::Nearest,
            mipmap_filter:      wgpu::FilterMode::Nearest,
            lod_min_clamp:      0.0,
            lod_max_clamp:      0.0,
            compare:            None,
            anisotropy_clamp:   None,
            border_color:       None,
        });

        RendererState {
            device:                             device,
            queue:                              command_queue,
            encoder:                            encoder,
            render_pass_resources:              RenderPassResources::default(),
            render_pass:                        vec![],
            pipeline_configuration:             PipelineConfiguration::default(),
            pipeline:                           None,
            pipeline_config_changed:            true,
            pipeline_bindings_changed:          true,
            active_pipeline_configuration:      None,

            target_size:                        (1, 1),
            texture_transform:                  texture_transform,
            active_matrix:                      Matrix::identity(),
            input_texture:                      None,
            clip_texture:                       None,
            sampler:                            Some(Arc::new(default_sampler)),
            texture_alpha:                      None,
        }
    }

    ///
    /// Updates the contents of the matrix buffer for this renderer
    ///
    #[inline]
    pub fn write_matrix(&mut self, new_matrix: &Matrix) {
        self.active_matrix = *new_matrix;
    }

    ///
    /// Updates the contents of the matrix buffer for this renderer
    ///
    #[inline]
    pub fn write_texture_transform(&mut self, new_transform: &Matrix) {
        self.texture_transform = Arc::new(Self::create_transform_buffer(&self.device, new_transform));
    }

    ///
    /// Creates a buffer containing a f32 value
    ///
    #[inline]
    pub fn f32_buffer(&self, value: f32) -> Arc<wgpu::Buffer> {
        let f32_buf     = [value];
        let f32_void    = f32_buf.as_ptr() as *const c_void;
        let f32_len     = mem::size_of::<[f32; 1]>();
        let f32_u8      = unsafe { slice::from_raw_parts(f32_void as *const u8, f32_len) };

        let f32_buffer  = self.device.create_buffer_init(&util::BufferInitDescriptor {
            label:      Some("matrix_buffer"),
            contents:   f32_u8,
            usage:      wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        Arc::new(f32_buffer)
    }

    ///
    /// Sets up the transform buffer and layout
    ///
    fn create_transform_buffer(device: &wgpu::Device, matrix: &Matrix) -> wgpu::Buffer {
        // Convert the matrix to a u8 pointer
        let matrix_void     = matrix.0.as_ptr() as *const c_void;
        let matrix_len      = mem::size_of::<[[f32; 4]; 4]>();
        let matrix_u8       = unsafe { slice::from_raw_parts(matrix_void as *const u8, matrix_len) };

        // Load into a buffer
        let matrix_buffer   = device.create_buffer_init(&util::BufferInitDescriptor {
            label:      Some("matrix_buffer"),
            contents:   matrix_u8,
            usage:      wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        matrix_buffer
    }

    ///
    /// Adds a step to the render pass to update to the currently set matrix
    ///
    pub fn bind_current_matrix(&mut self) {
        if let Some(pipeline) = &self.pipeline {
            // Add the matrix to the buffer
            let matrix_buffer_index     = self.render_pass_resources.matrices.len();
            let matrix_group            = pipeline.matrix_group_index();
            self.render_pass_resources.matrices.push(self.active_matrix.0);

            // Bind the matrix as the next step in the pending render pass
            self.render_pass.push(Box::new(move |resources, render_pass| {
                render_pass.set_bind_group(matrix_group, &resources.matrix_bind_groups[matrix_buffer_index], &[]);
            }));
        }
    }

    ///
    /// Adds a step to the render pass to update to the current set clip mask
    ///
    pub fn bind_current_clip_mask(&mut self) {
        if let Some(pipeline) = &self.pipeline {
            let clip_texture    = self.clip_texture.clone();

            // Set up the clip binding
            let clip_group      = pipeline.clip_mask_group_index();
            let clip_binding    = pipeline.bind_clip_mask(&*self.device, clip_texture.as_ref().map(|clip_texture| &**clip_texture));
            let clip_index      = self.render_pass_resources.bind_groups.len();

            // Store in the render pass resources so it's not freed before then
            self.render_pass_resources.bind_groups.push(Arc::new(clip_binding));
            if let Some(clip_texture) = clip_texture {
                self.render_pass_resources.textures.push(clip_texture);
            }

            // Bind as the next step in the pending render pass
            self.render_pass.push(Box::new(move |resources, render_pass| {
                render_pass.set_bind_group(clip_group, &resources.bind_groups[clip_index], &[]);
            }));
        }
    }

    ///
    /// Adds a step to the render pass to update to the currently set texture
    ///
    pub fn bind_current_texture(&mut self) {
        if let Some(pipeline) = &self.pipeline {
            // Fetch the texture state
            let texture_transform   = self.texture_transform.clone();
            let input_texture       = self.input_texture.clone();
            let sampler             = self.sampler.clone();
            let texture_alpha       = self.texture_alpha.clone();

            // Set up the texture binding
            let texture_group   = pipeline.input_texture_group_index();
            let texture_binding = pipeline.bind_input_texture(&*self.device, &*texture_transform, input_texture.as_ref().map(|t| &**t), sampler.as_ref().map(|s| &**s), texture_alpha.as_ref().map(|b| &**b));
            let texture_index   = self.render_pass_resources.bind_groups.len();

            self.render_pass_resources.bind_groups.push(Arc::new(texture_binding));
            self.render_pass_resources.buffers.push(texture_transform);
            if let Some(input_texture) = input_texture  { self.render_pass_resources.textures.push(input_texture); }
            if let Some(sampler) = sampler              { self.render_pass_resources.samplers.push(sampler); }
            if let Some(texture_alpha) = texture_alpha  { self.render_pass_resources.buffers.push(texture_alpha); }

            // Add a callback function to actually set up the render pipeline (we have to do it indirectly later on because it borrows its resources)
            self.render_pass.push(Box::new(move |resources, render_pass| {
                render_pass.set_bind_group(texture_group, &resources.bind_groups[texture_index], &[]);
            }));
        }
    }

    ///
    /// Runs the pending render pass
    ///
    pub fn run_render_pass(&mut self) {
        // Take the actions and the resources for this render pass
        let render_actions  = mem::take(&mut self.render_pass);
        let mut resources   = mem::take(&mut self.render_pass_resources);

        // Keep the current texture view for the next render pass
        self.render_pass_resources.target_view  = resources.target_view.clone();

        // This resets the active pipeline configuration
        self.active_pipeline_configuration      = None;
        self.pipeline_config_changed            = true;

        // Abort early if there are no render actions
        if render_actions.is_empty() {
            return;
        }

        // Start a new render pass using the current encoder
        if let Some(texture_view) = &resources.target_view {
            // Create any buffers required
            resources.fill_matrix_buffer(&*self.device, self.pipeline.as_ref().unwrap());

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

        // Commit the commands that are pending in the command encoder
        // It's probably not the most efficient way to do things, but it simplifies resource management 
        // a lot (we'll need to hold on to all of the resources from the render pass resources until this
        // is done otherwise). Might be some advantage to committing some commands to the GPU while we
        // generate more too.
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("run_render_pass") });
        mem::swap(&mut encoder, &mut self.encoder);

        self.queue.submit(Some(encoder.finish()));
    }
}