use wgpu;

use std::sync::*;

///
/// Representation of a texture stored in the WGPU renderer
///
pub (crate) struct WgpuTexture {
    /// The WGPU texture stored here
    pub texture: Arc<wgpu::Texture>,

    /// True if this texture has premultiplied alpha
    pub is_premultiplied: bool,
}
