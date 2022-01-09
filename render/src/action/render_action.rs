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

impl RenderAction {
    ///
    /// Provides a description of this action without the full details (similar to using the Debug trait, but won't show the full list of vertices)
    ///
    pub fn short_description(&self) -> String {
        use self::RenderAction::*;

        match self {
            SetTransform(matrix)                                            => format!("SetTransform({:?})", matrix),
            CreateVertex2DBuffer(buffer_id, vertices)                       => format!("CreateVertex2DBuffer({:?}, [{} vertices])", buffer_id, vertices.len()),
            CreateIndexBuffer(buffer_id, indexes)                           => format!("CreateIndexBuffer({:?}, [{} indexes])", buffer_id, indexes.len()),
            FreeVertexBuffer(buffer_id)                                     => format!("FreeVertexBuffer({:?})", buffer_id),
            FreeIndexBuffer(buffer_id)                                      => format!("FreeIndexBuffer({:?})", buffer_id),
            BlendMode(blend_mode)                                           => format!("BlendMode({:?})", blend_mode),
            CreateRenderTarget(render_id, texture_id, w, h, target_type)    => format!("CreateRenderTarget({:?}, {:?}, {:?}, {:?}, {:?})", render_id, texture_id, w, h, target_type),
            FreeRenderTarget(render_id)                                     => format!("FreeRenderTarget({:?})", render_id),
            SelectRenderTarget(render_id)                                   => format!("SelectRenderTarget({:?})", render_id),
            RenderToFrameBuffer                                             => format!("RenderToFrameBuffer"),
            ShowFrameBuffer                                                 => format!("ShowFrameBuffer"),
            DrawFrameBuffer(render_id, x, y)                                => format!("DrawFrameBuffer({:?}, {:?}, {:?})", render_id, x, y),
            CreateTextureBgra(texture_id, w, h)                             => format!("CreateTextureBgra({:?}, {:?}, {:?})", texture_id, w, h),
            CreateTextureMono(texture_id, w, h)                             => format!("CreateTextureMono({:?}, {:?}, {:?})", texture_id, w, h),
            Create1DTextureBgra(texture_id, w)                              => format!("Create1DTextureBgra({:?}, {:?})", texture_id, w),
            Create1DTextureMono(texture_id, w)                              => format!("Create1DTextureMono({:?}, {:?})", texture_id, w),
            WriteTextureData(texture_id, (x, y), (w, h), bytes)             => format!("WriteTextureData({:?}, ({:?}, {:?}), ({:?}, {:?}), [{} bytes])", texture_id, x, y, w, h, bytes.len()),
            WriteTexture1D(texture_id, x, w, bytes)                         => format!("WriteTexture1D({:?}, {:?}, {:?}, [{} bytes])", texture_id, x, w, bytes.len()),
            CreateMipMaps(texture_id)                                       => format!("CreateMipMaps({:?})", texture_id),
            CopyTexture(id1, id2)                                           => format!("CopyTexture({:?}, {:?})", id1, id2),
            FreeTexture(texture_id)                                         => format!("FreeTexture({:?})", texture_id),
            Clear(bg_col)                                                   => format!("Clear({:?})", bg_col),
            UseShader(shader_type)                                          => format!("UseShader({:?})", shader_type),
            DrawTriangles(buffer_id, range)                                 => format!("DrawTriangles({:?}, {:?})", buffer_id, range),
            DrawIndexedTriangles(buffer_id, index_id, len)                  => format!("DrawIndexedTriangles({:?}, {:?}, {:?})", buffer_id, index_id, len),
        }
    }
}
