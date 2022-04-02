use wgpu;

use std::sync::*;

///
/// Renderer that uses the `wgpu` abstract library as a render target
///
pub struct WgpuRenderer {
    /// A reference to the device that this will render to
    device: Arc<wgpu::Device>
}

