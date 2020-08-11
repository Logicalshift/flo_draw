use super::buffer::*;

use crate::action::*;
use crate::buffer::*;

use metal;

use std::ops::{Range};

///
/// Renderer that can write to a surface using Apple's Metal API
///
pub struct MetalRenderer {
    /// The device that this will render to
    device: metal::Device,

    /// The vertex buffers allocated to this renderer
    vertex_buffers: Vec<Option<Buffer>>,

    /// The index buffers defined for this renderer
    index_buffers: Vec<Option<Buffer>>
}

impl MetalRenderer {
    ///
    /// Creates a new metal renderer using the system default device
    ///
    pub fn with_default_device() -> MetalRenderer {
        let device = metal::Device::system_default().expect("No Metal device available");

        MetalRenderer {
            device:         device,
            vertex_buffers: vec![],
            index_buffers:  vec![]
        }
    }

    ///
    /// Performs rendering of the specified actions to this device target
    ///
    pub fn render<Actions: IntoIterator<Item=RenderAction>>(&mut self, actions: Actions, target_drawable: &metal::Drawable) {
        for action in actions {
            use self::RenderAction::*;

            match action {
                SetTransform(matrix)                                                    => { self.set_transform(matrix); }
                CreateVertex2DBuffer(id, vertices)                                      => { self.create_vertex_buffer_2d(id, vertices); }
                CreateIndexBuffer(id, indices)                                          => { self.create_index_buffer(id, indices); }
                FreeVertexBuffer(id)                                                    => { self.free_vertex_buffer(id); }
                BlendMode(blend_mode)                                                   => { self.blend_mode(blend_mode); }
                CreateRenderTarget(render_id, texture_id, width, height, render_type)   => { self.create_render_target(render_id, texture_id, width, height, render_type); }
                FreeRenderTarget(render_id)                                             => { self.free_render_target(render_id); }
                SelectRenderTarget(render_id)                                           => { self.select_render_target(render_id); }
                RenderToFrameBuffer                                                     => { self.select_main_frame_buffer(); }
                DrawFrameBuffer(render_id, x, y)                                        => { self.draw_frame_buffer(render_id, x, y); }
                ShowFrameBuffer                                                         => { /* This doesn't double-buffer so nothing to do */ }
                CreateTextureBgra(texture_id, width, height)                            => { self.create_bgra_texture(texture_id, width, height); }
                FreeTexture(texture_id)                                                 => { self.free_texture(texture_id); }
                Clear(color)                                                            => { self.clear(color); }
                UseShader(shader_type)                                                  => { self.use_shader(shader_type); }
                DrawTriangles(buffer_id, buffer_range)                                  => { self.draw_triangles(buffer_id, buffer_range); }
                DrawIndexedTriangles(vertex_buffer, index_buffer, num_vertices)         => { self.draw_indexed_triangles(vertex_buffer, index_buffer, num_vertices); }
            }
        }
    }

    fn set_transform(&mut self, matrix: Matrix) {

    }

    ///
    /// Loads a vertex buffer and associates it with an ID
    ///
    fn create_vertex_buffer_2d(&mut self, VertexBufferId(vertex_id): VertexBufferId, vertices: Vec<Vertex2D>) {
        // Reserve space for the buffer ID
        if vertex_id >= self.vertex_buffers.len() {
            self.vertex_buffers.extend((self.vertex_buffers.len()..(vertex_id+1))
                .into_iter()
                .map(|_| None));
        }

        // Free any existing buffer
        self.vertex_buffers[vertex_id] = None;

        // Load and store the new buffer
        self.vertex_buffers[vertex_id] = Some(Buffer::from_vertices(&self.device, vertices));
    }

    ///
    /// Loads an index buffer and associates it with an ID
    ///
    fn create_index_buffer(&mut self, IndexBufferId(index_id): IndexBufferId, indices: Vec<u16>) {
        // Reserve space for the buffer ID
        if index_id >= self.index_buffers.len() {
            self.index_buffers.extend((self.index_buffers.len()..(index_id+1))
                .into_iter()
                .map(|_| None));
        }

        // Free any existing buffer
        self.index_buffers[index_id] = None;

        // Load and store the new buffer
        self.index_buffers[index_id] = Some(Buffer::from_indices(&self.device, indices));
    }

    fn free_vertex_buffer(&mut self, VertexBufferId(vertex_id): VertexBufferId) {
        self.vertex_buffers[vertex_id] = None;
    }

    fn blend_mode(&mut self, blend_mode: BlendMode) {

    }

    fn create_render_target(&mut self, RenderTargetId(render_id): RenderTargetId, TextureId(texture_id): TextureId, width: usize, height: usize, render_target_type: RenderTargetType) {

    }

    fn free_render_target(&mut self, RenderTargetId(render_id): RenderTargetId) {

    }

    fn select_render_target(&mut self, RenderTargetId(render_id): RenderTargetId) {

    }

    fn select_main_frame_buffer(&mut self) {

    }

    fn draw_frame_buffer(&mut self, RenderTargetId(source_buffer): RenderTargetId, x: i32, y: i32) {

    }

    fn create_bgra_texture(&mut self, TextureId(texture_id): TextureId, width: usize, height: usize) {

    }

    fn free_texture(&mut self, TextureId(texture_id): TextureId) {

    }

    fn clear(&mut self, color: Rgba8) {

    }

    fn use_shader(&mut self, shader_type: ShaderType) {

    }

    fn draw_triangles(&mut self, VertexBufferId(vertex_buffer_id): VertexBufferId, range: Range<usize>) {

    }

    fn draw_indexed_triangles(&mut self, VertexBufferId(vertex_buffer_id): VertexBufferId, IndexBufferId(index_buffer_id): IndexBufferId, num_vertices: usize) {

    }
}
