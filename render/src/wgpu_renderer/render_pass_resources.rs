use wgpu;

use std::sync::*;

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
            clear:              None,
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
}
