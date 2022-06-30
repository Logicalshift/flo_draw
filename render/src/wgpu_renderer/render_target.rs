use wgpu;

use std::sync::*;

///
/// Represents a WGPU render target
///
pub enum RenderTarget {
    /// Surface
    Surface {
        surface:    Arc<wgpu::Surface>,
        width:      usize,
        height:     usize,
    },

    /// Simple texture
    Texture {
        texture:        Arc<wgpu::Texture>,
        texture_format: wgpu::TextureFormat,
        width:          usize,
        height:         usize,
    },

    /// Multisampled texture
    Multisampled {
        texture:        Arc<wgpu::Texture>,
        texture_format: wgpu::TextureFormat,
        resolved:       Option<Arc<wgpu::Texture>>,
        width:          usize,
        height:         usize,
    },
}
