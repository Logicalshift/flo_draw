use wgpu;

use std::sync::*;

///
/// Representation of a texture stored in the WGPU renderer
///
pub (crate) struct WgpuTexture {
    /// The descriptor used to create the texture
    pub descriptor: wgpu::TextureDescriptor<'static>,

    /// The WGPU texture stored here
    pub texture: Arc<wgpu::Texture>,

    /// True if this texture has premultiplied alpha
    pub is_premultiplied: bool,
}
