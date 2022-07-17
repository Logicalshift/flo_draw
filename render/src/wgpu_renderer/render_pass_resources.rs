use super::pipeline::*;

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

    /// The samplers that this render pass will read from
    pub (crate) samplers: Vec<Arc<wgpu::Sampler>>,

    /// If set to a colour, sets what the render target will be cleared to at the start of the pass
    pub (crate) clear: Option<wgpu::Color>,

    /// The matrices that will be loaded into the matrix buffer for this render pass
    pub (crate) matrices: Vec<[[f32; 4]; 4]>,

    /// Once the render pass is running, the buffer containing the matrices that were previously in 'matrices'
    pub (crate) matrix_buffer: Option<wgpu::Buffer>,

    /// The bind groups for each of the matrices in the matrix buffer (corresponding to the original index in the matrices Vec)
    pub (crate) matrix_bind_groups: Vec<wgpu::BindGroup>,
}

impl Default for RenderPassResources {
    fn default() -> RenderPassResources {
        RenderPassResources {
            target_texture:     None,
            target_view:        None,
            pipelines:          vec![],
            buffers:            vec![],
            bind_groups:        vec![],
            textures:           vec![],
            samplers:           vec![],
            matrices:           vec![],
            clear:              None,
            matrix_buffer:      None,
            matrix_bind_groups: vec![],
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
        let matrices = mem::take(&mut self.matrices);

        // Convert the matrix to a u8 pointer
        let matrix_void     = matrices.as_ptr() as *const c_void;
        let matrix_len      = mem::size_of::<[[f32; 4]; 4]>() * matrices.len();
        let matrix_u8       = unsafe { slice::from_raw_parts(matrix_void as *const u8, matrix_len) };

        // Load into a buffer
        let matrix_buffer   = device.create_buffer_init(&util::BufferInitDescriptor {
            label:      Some("fill_matrix_buffer"),
            contents:   matrix_u8,
            usage:      wgpu::BufferUsages::UNIFORM,
        });

        // Create bind groups for each of the matrices in the buffer
        let bind_groups = (0..matrices.len()).into_iter()
            .map(|offset| pipeline.bind_matrix_buffer(device, &matrix_buffer, offset))
            .collect();

        // Store the matrix buffer for use during the render pass
        self.matrix_buffer      = Some(matrix_buffer);
        self.matrix_bind_groups = bind_groups;
    }
}
