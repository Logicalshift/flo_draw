use super::color::*;
use super::identities::*;
use super::blend_mode::*;
use super::shader_type::*;
use super::render_target_type::*;

use crate::buffer::*;

use std::ops::{Range};
use std::sync::*;

///
/// Represents an action for a render target
///
#[derive(Clone, PartialEq, Debug)]
pub enum RenderAction {
    ///
    /// Sets the transformation matrix to use for future renderings
    ///
    SetTransform(Matrix),

    ///
    /// Creates a vertex buffer with the specified 2D vertices in it (replacing any existing buffer)
    ///
    CreateVertex2DBuffer(VertexBufferId, Vec<Vertex2D>),

    ///
    /// Creates an index buffer with the specified 2D vertices in it (replacing any existing buffer)
    ///
    CreateIndexBuffer(IndexBufferId, Vec<u16>),

    ///
    /// Frees an existing vertex buffer
    ///
    FreeVertexBuffer(VertexBufferId),

    ///
    /// Frees an existing index buffer
    ///
    FreeIndexBuffer(IndexBufferId),

    ///
    /// Sets the blend mode for future drawing operations (SourceOver is the default)
    ///
    BlendMode(BlendMode),

    ///
    /// Creates a new render target of the specified size, as the specified texture
    ///
    CreateRenderTarget(RenderTargetId, TextureId, usize, usize, RenderTargetType),

    ///
    /// Frees up an existing render target
    ///
    FreeRenderTarget(RenderTargetId),

    ///
    /// Send future rendering instructions to the specified render target
    ///
    SelectRenderTarget(RenderTargetId),

    ///
    /// Send future rendering instructions to the main frame buffer
    ///
    RenderToFrameBuffer,

    ///
    /// Display the current frame buffer on-screen
    ///
    ShowFrameBuffer,

    ///
    /// Renders the specified framebuffer to the current framebuffer
    ///
    DrawFrameBuffer(RenderTargetId, i32, i32),

    ///
    /// Creates an 8-bit BGRA 2D texture of the specified size
    ///
    CreateTextureBgra(TextureId, usize, usize),

    ///
    /// Creates an 8-bit monochrome 2D texture of the specified size
    ///
    CreateTextureMono(TextureId, usize, usize),

    ///
    /// Creates a 1 dimensional 8-bit BGRA texture of the specified size
    ///
    Create1DTextureBgra(TextureId, usize),

    ///
    /// Creates a 1 dimensional 8-bit monochrome texture of the specified size
    ///
    Create1DTextureMono(TextureId, usize),

    ///
    /// Given a region in a 2D texture and a set of bytes to write, updates the texture with those bytes
    ///
    WriteTextureData(TextureId, (usize, usize), (usize, usize), Arc<Vec<u8>>),

    ///
    /// Given a region in a 1D texture and a set of bytes to write, updates the texture with those bytes
    ///
    WriteTexture1D(TextureId, usize, usize, Arc<Vec<u8>>),

    ///
    /// Generates mip-maps for the specified texture ID
    ///
    CreateMipMaps(TextureId),

    ///
    /// Copies a texture from a source ID to a target ID (replacing any existing texture at that ID)
    ///
    /// Mipmap levels are not copied by this operation, so would need to be regenerated
    ///
    CopyTexture(TextureId, TextureId),

    ///
    /// Frees up an existing texture
    ///
    FreeTexture(TextureId),

    ///
    /// Clears the current render target to the specified colour
    ///
    Clear(Rgba8),

    ///
    /// Uses the specified shader
    ///
    UseShader(ShaderType),

    ///
    /// Renders triangles from a vertex buffer (with no texture)
    ///
    /// Parameters are the range of vertices to use
    ///
    DrawTriangles(VertexBufferId, Range<usize>),

    ///
    /// Renders triangles using an index buffer
    ///
    DrawIndexedTriangles(VertexBufferId, IndexBufferId, usize)
}
