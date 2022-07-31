use super::pipeline::*;
use super::texture_settings::*;

use wgpu;
use wgpu::util;
use wgpu::util::{DeviceExt};

use std::mem;
use std::slice;
use std::sync::*;
use std::ffi::{c_void};

///
/// Resources used for the current render pass (required for lifetime bookkeeping due to the design of WGPU)
///
/// This is needed as a WGPU render pass borrows its resources rather than retaining a reference for them
/// (probably due to performance, but a bit of an own goal due to need for something like this). Render passes
/// can be built up using the `current_render_pass` field of the renderer state to define functions that
/// run against the render pass (created later on due to the borrowing requirement), and also store and
/// later borrow their resources from here in order to fulfil the lifetime requirements of the render
/// pass itself.
///
pub struct RenderPassResources {
    /// The texture that this render pass will write to
    pub (crate) target_texture: Option<Arc<wgpu::Texture>>,

    /// The texture view that this render pass will write to
    pub (crate) target_view: Option<Arc<wgpu::TextureView>>,

    /// The render pipelines that this render pass will write to
    pub (crate) pipelines: Vec<Arc<wgpu::RenderPipeline>>,

    /// Cache of the buffers used by the render pass. When adding a buffer to the cache, always add to the end,
    /// assume that rendering operations have cached the location of their own resources.
    pub (crate) buffers: Vec<Arc<wgpu::Buffer>>,

    /// Cache of the bind groups used by the render pass.
    pub (crate) bind_groups: Vec<Arc<wgpu::BindGroup>>,

    /// The textures that this render pass will read from
    pub (crate) textures: Vec<Arc<wgpu::Texture>>,

    /// If set to a colour, sets what the render target will be cleared to at the start of the pass
    pub (crate) clear: Option<wgpu::Color>,

    /// The matrices that will be loaded into the matrix buffer for this render pass
    pub (crate) matrices: Vec<[[f32; 4]; 4]>,

    /// Once the render pass is running, the buffer containing the matrices that were previously in 'matrices'
    pub (crate) matrix_buffer: Option<wgpu::Buffer>,

    /// Once the render pass is running, the bind groups for each of the matrices in the matrix buffer (corresponding to the original index in the matrices Vec)
    pub (crate) matrix_bind_groups: Vec<wgpu::BindGroup>,

    /// The texture settings that will be loaded into the texture settings buffer for this render pass
    pub (crate) texture_settings: Vec<(Arc<Pipeline>, TextureSettings, Option<Arc<wgpu::Texture>>, Option<Arc<wgpu::Sampler>>)>,

    /// Once the render pass is running, the buffer containing all of the texture settings from the texture_settings Vec
    pub (crate) texture_settings_buffer: Option<wgpu::Buffer>,

    /// Once the render pass is running, the bind groups for each of the texture settings in the texture settings buffer (corresponding to the original index in the texture_settings Vec)
    pub (crate) texture_settings_bind_groups: Vec<wgpu::BindGroup>,
}

impl Default for RenderPassResources {
    fn default() -> RenderPassResources {
        RenderPassResources {
            target_texture:                 None,
            target_view:                    None,
            pipelines:                      vec![],
            buffers:                        vec![],
            bind_groups:                    vec![],
            textures:                       vec![],
            matrices:                       vec![],
            texture_settings:               vec![],
            clear:                          None,
            matrix_buffer:                  None,
            matrix_bind_groups:             vec![],
            texture_settings_buffer:        None,
            texture_settings_bind_groups:   vec![],
        }
    }
}

impl RenderPassResources {
    ///
    /// Generates the colour attachments for the render pass for these resources
    ///
    #[inline]
    pub fn color_attachments(&self) -> Vec<Option<wgpu::RenderPassColorAttachment>> {
        let load_op = if let Some(clear_color) = self.clear {
            wgpu::LoadOp::Clear(clear_color)
        } else {
            wgpu::LoadOp::Load
        };

        if let Some(target_view) = &self.target_view {
            vec![
                Some(wgpu::RenderPassColorAttachment {
                    view:           &**target_view,
                    resolve_target: None,
                    ops:            wgpu::Operations { load: load_op, store: true }
                })
            ]
        } else {
            vec![]
        }
    }

    ///
    /// Loads the matrices in this render pass into the matrix_buffer object
    ///
    pub (crate) fn fill_matrix_buffer(&mut self, device: &wgpu::Device, pipeline: &Pipeline) {
        // Take the matrices in preparation to load them into the buffer
        let matrices    = mem::take(&mut self.matrices);
        let matrix_size = mem::size_of::<[[f32; 4]; 4]>();

        // Convert the matrix list to a u8 pointer
        let matrices_void     = matrices.as_ptr() as *const c_void;
        let matrices_len      = matrix_size * matrices.len();
        let matrices_u8       = unsafe { slice::from_raw_parts(matrices_void as *const u8, matrices_len) };

        // Need to make another buffer from this where everything is aligned according to the min_uniform_buffer_offset_alignment
        let limits              = device.limits();
        let mut group_offset    = 0;

        while group_offset < mem::size_of::<[[f32; 4]; 4]>() {
            group_offset += limits.min_uniform_buffer_offset_alignment as usize;
        }

        // Copy the matrices aligned at group_offset bytes
        let mut aligned_matrices = vec![0; group_offset * matrices.len()];
        for matrix_num in 0..matrices.len() {
            let original_offset = matrix_size * matrix_num;
            let new_offset      = group_offset * matrix_num;

            aligned_matrices[new_offset..(new_offset + matrix_size)].copy_from_slice(&matrices_u8[original_offset..(original_offset + matrix_size)]);
        }

        // Load into a buffer
        let matrix_buffer   = device.create_buffer_init(&util::BufferInitDescriptor {
            label:      Some("fill_matrix_buffer"),
            contents:   &aligned_matrices,
            usage:      wgpu::BufferUsages::UNIFORM,
        });

        // Create bind groups for each of the matrices in the buffer
        let bind_groups = (0..matrices.len()).into_iter()
            .map(|offset| pipeline.bind_matrix_buffer(device, &matrix_buffer, offset * group_offset))
            .collect();

        // Store the matrix buffer for use during the render pass
        self.matrix_buffer      = Some(matrix_buffer);
        self.matrix_bind_groups = bind_groups;
    }

    ///
    /// Loads the texture settings in this render pass into the texture_settings_buffer object
    ///
    pub (crate) fn fill_texture_settings_buffer(&mut self, device: &wgpu::Device) {
        // Take the matrices in preparation to load them into the buffer
        let settings        = mem::take(&mut self.texture_settings);
        let settings_size   = mem::size_of::<TextureSettings>();

        // Need to make another buffer from this where everything is aligned according to the min_uniform_buffer_offset_alignment
        let limits              = device.limits();
        let mut group_offset    = 0;

        while group_offset < settings_size {
            group_offset += limits.min_uniform_buffer_offset_alignment as usize;
        }

        // Copy the texture settings aligned at group_offset bytes
        let mut aligned_settings = vec![0; group_offset * settings.len()];
        for setting_num in 0..settings.len() {
            // Create a buffer containing the setting
            let (_, texture_setting, _, _) = &settings[setting_num];

            let settings_void   = texture_setting as *const _ as *const c_void;
            let settings_u8     = unsafe { slice::from_raw_parts(settings_void as *const u8, settings_size) };

            // Copy the setting into the aligned settings
            let new_offset      = group_offset * setting_num;
            aligned_settings[new_offset..(new_offset + settings_size)].copy_from_slice(&settings_u8[..]);
        }

        // Load into a buffer
        let settings_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
            label:      Some("fill_texture_settings_buffer"),
            contents:   &aligned_settings,
            usage:      wgpu::BufferUsages::UNIFORM,
        });

        // Create bind groups for each of the texture settings in the buffer
        let bind_groups = (0..settings.len()).into_iter()
            .map(|setting_num| {
                let (pipeline, _, texture, sampler) = &settings[setting_num];
                pipeline.bind_input_texture(device, &settings_buffer, setting_num * group_offset, texture.as_ref().map(|t| &**t), sampler.as_ref().map(|s| &**s))
            })
            .collect();

        // Store the settings buffer for use during the render pass
        self.texture_settings_buffer        = Some(settings_buffer);
        self.texture_settings_bind_groups   = bind_groups;
    }
}
