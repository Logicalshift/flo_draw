use super::pipeline::*;
use super::texture_settings::*;
use super::render_pass_resources::*;
use super::pipeline_configuration::*;
use crate::buffer::*;

use wgpu;

use std::mem;
use std::sync::*;

///
/// State for the WGPU renderer
///
pub (crate) struct RendererState {
    /// The device this will render to
    device:                             Arc<wgpu::Device>,

    /// The command queue for the device
    pub queue:                          Arc<wgpu::Queue>,

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

    /// True if the pipeline bindings have been updated since they were last written (all will be updated if this is true)
    pub pipeline_bindings_changed:      bool,

    /// True if the pipeline matrix has changed since it was last written (only the matrix bindings will be updated if this is true)
    pub pipeline_matrix_changed:        bool,

    /// The pipeline configuration that was last activated
    pub active_pipeline_configuration:  Option<PipelineConfiguration>,

    /// The actions for the active render pass (deferred so we can manage the render pass lifetime)
    pub render_pass:                    Vec<Box<dyn for<'a> FnOnce(&'a RenderPassResources, &mut wgpu::RenderPass<'a>) -> ()>>,

    /// The size of the current render target
    pub target_size:                    (u32, u32),

    /// The last transform matrix set
    pub active_matrix:                  Matrix,

    /// The current texture settings
    pub texture_settings:               TextureSettings,

    /// The input texture set for the current shader (or none)
    pub input_texture:                  Option<Arc<wgpu::Texture>>,

    /// The texture used for clipping the image
    pub clip_texture:                   Option<Arc<wgpu::Texture>>,

    /// The sampler for the current shader (or none)
    pub sampler:                        Option<Arc<wgpu::Sampler>>,

    /// The texture to present to the surface once the rendering is done
    pub present:                        Option<wgpu::SurfaceTexture>,
}

impl RendererState {
    ///
    /// Creates a default render state
    ///
    pub fn new(command_queue: Arc<wgpu::Queue>, device: Arc<wgpu::Device>) -> RendererState {
        // TODO: we can avoid re-creating some of these structures every frame: eg, the binding groups in particular

        // Create all the state structures
        let encoder             = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("RendererState::new") });

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
            pipeline_matrix_changed:            true,
            active_pipeline_configuration:      None,

            target_size:                        (1, 1),
            active_matrix:                      Matrix::identity(),
            texture_settings:                   TextureSettings { transform: Matrix::identity().0, alpha: 1.0, ..Default::default() },
            input_texture:                      None,
            clip_texture:                       None,
            sampler:                            None,
            present:                            None,
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
    /// Adds a step to the render pass to update to the currently set matrix
    ///
    pub fn bind_current_matrix(&mut self) {
        if let Some(pipeline) = &self.pipeline {
            // Add the matrix to the buffer
            let matrix_buffer_index     = self.render_pass_resources.matrices.len();
            let matrix_group            = pipeline.matrix_group_index();

            let mut active_matrix       = self.active_matrix.0;
            if pipeline.flip_vertical {
                active_matrix = [
                    [active_matrix[0][0], active_matrix[0][1], active_matrix[0][2], active_matrix[0][3]],
                    [-active_matrix[1][0], -active_matrix[1][1], -active_matrix[1][2], -active_matrix[1][3]],
                    [active_matrix[2][0], active_matrix[2][1], active_matrix[2][2], active_matrix[2][3]],
                    [active_matrix[3][0], active_matrix[3][1], active_matrix[3][2], active_matrix[3][3]],
                ];
            }
            self.render_pass_resources.matrices.push(active_matrix);

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
            let texture_settings    = self.texture_settings;
            let input_texture       = self.input_texture.clone();
            let sampler             = self.sampler.clone();

            // Set up the texture binding
            let settings_buffer_index   = self.render_pass_resources.texture_settings.len();
            let texture_group           = pipeline.input_texture_group_index();
            self.render_pass_resources.texture_settings.push((pipeline.clone(), texture_settings, input_texture, sampler));

            // Add a callback function to actually set up the render pipeline (we have to do it indirectly later on because it borrows its resources)
            self.render_pass.push(Box::new(move |resources, render_pass| {
                render_pass.set_bind_group(texture_group, &resources.texture_settings_bind_groups[settings_buffer_index], &[]);
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
        if resources.target_view.is_some() {
            // Create any buffers required
            resources.fill_matrix_buffer(&*self.device, self.pipeline.as_ref().unwrap());
            resources.fill_texture_settings_buffer(&*self.device);

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