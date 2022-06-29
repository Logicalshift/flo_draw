use super::wgpu_shader::*;
use super::render_target::*;

use wgpu;

use std::sync::*;
use std::collections::{HashMap};

///
/// Renderer that uses the `wgpu` abstract library as a render target
///
pub struct WgpuRenderer {
    /// A reference to the device that this will render to
    device: Arc<wgpu::Device>,

    /// The command queue for the device
    queue: Arc<wgpu::Queue>,

    /// The surface that this renderer will target
    target_surface: Arc<wgpu::Surface>,

    /// The shaders that have been loaded for this renderer
    shaders: HashMap<WgpuShader, Arc<wgpu::ShaderModule>>,

    /// The vertex buffers for this renderer
    vertex_buffers: Vec<Option<wgpu::Buffer>>,

    /// The index buffers for this renderer
    index_buffers: Vec<Option<wgpu::Buffer>>,

    /// The textures for this renderer
    textures: Vec<Option<Arc<wgpu::Texture>>>,

    /// The render targets for this renderer
    render_targets: Vec<Option<RenderTarget>>,
}

impl WgpuRenderer {
    ///
    /// Creates a new WGPU renderer
    ///
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>, target_surface: Arc<wgpu::Surface>) -> WgpuRenderer {
        WgpuRenderer {
            device:         device,
            queue:          queue,
            target_surface: target_surface,
            shaders:        HashMap::new(),
            vertex_buffers: vec![],
            index_buffers:  vec![],
            textures:       vec![],
            render_targets: vec![],
        }
    }
}
