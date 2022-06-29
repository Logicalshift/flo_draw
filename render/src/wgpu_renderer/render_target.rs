use wgpu;

use std::sync::*;

///
/// Represents a WGPU render target
///
pub enum RenderTarget {
    /// Simple texture
    Texture {
        texture:    Arc<wgpu::Texture>,
        width:      usize,
        height:     usize,
    },

    /// Multisampled texture
    Multisampled {
        texture:    Arc<wgpu::Texture>,
        resolved:   Option<Arc<wgpu::Texture>>,
        width:      usize,
        height:     usize,
    },
}
