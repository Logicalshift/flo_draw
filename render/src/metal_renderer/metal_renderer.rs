use super::buffer::*;
use super::matrix_buffer::*;
use super::pipeline_configuration::*;

use crate::action::*;
use crate::buffer::*;

use metal;

use std::ops::{Range};
use std::collections::{HashMap};

///
/// Renderer that can write to a surface using Apple's Metal API
///
pub struct MetalRenderer {
    /// The device that this will render to
    device: metal::Device,

    /// The shader library for this renderer
    shader_library: metal::Library,

    /// The command queue we're using to render to this device
    command_queue: metal::CommandQueue,

    /// The vertex buffers allocated to this renderer
    vertex_buffers: Vec<Option<Buffer>>,

    /// The index buffers defined for this renderer
    index_buffers: Vec<Option<Buffer>>,

    /// The cache of render pipeline states used by this renderer
    pipeline_states: HashMap<PipelineConfiguration, metal::RenderPipelineState>
}

///
/// The current state of a renderer
///
struct RenderState<'a> {
    /// The main render buffer
    main_buffer: &'a metal::Drawable,

    /// The current target render buffer
    target_buffer: metal::Drawable,

    /// Buffer containing the current transformation matrix
    matrix: MatrixBuffer,

    /// The active pipeline configuration
    pipeline_config: PipelineConfiguration,

    /// The active pipeline state corresponding to the pipeline configuration
    pipeline_state: metal::RenderPipelineState,

    /// The command buffer we're using to send rendering actions
    command_buffer: &'a metal::CommandBufferRef
}

impl MetalRenderer {
    ///
    /// Creates a new metal renderer using the system default device
    ///
    pub fn with_default_device() -> MetalRenderer {
        let device          = metal::Device::system_default().expect("No Metal device available");
        let command_queue   = device.new_command_queue();
        let shader_library  = device.new_library_with_data(include_bytes![concat!(env!("OUT_DIR"), "/flo.metallib")]).unwrap();

        MetalRenderer {
            device:             device,
            command_queue:      command_queue,
            vertex_buffers:     vec![],
            shader_library:     shader_library,
            index_buffers:      vec![],
            pipeline_states:    HashMap::new()
        }
    }

    ///
    /// Returns a pipeline state for a configuration
    ///
    fn get_pipeline_state(&mut self, config: &PipelineConfiguration) -> metal::RenderPipelineState {
        // Borrow the fields
        let pipeline_states = &mut self.pipeline_states;
        let device          = &self.device;
        let shader_library  = &self.shader_library;

        // Retrieve the pipeline state for this configuration
        if let Some(pipeline) = pipeline_states.get(config) {
            pipeline.clone()
        } else {
            let pipeline = config.to_pipeline_state(&device, &shader_library);
            pipeline_states.insert(config.clone(), pipeline.clone());

            pipeline
        }
    }

    ///
    /// Performs rendering of the specified actions to this device target
    ///
    pub fn render<Actions: IntoIterator<Item=RenderAction>>(&mut self, actions: Actions, target_drawable: &metal::Drawable) {
        // Create the render state
        let command_queue       = self.command_queue.clone();
        let matrix              = MatrixBuffer::from_matrix(&self.device, Matrix::identity());
        let pipeline_config     = PipelineConfiguration::default();
        let pipeline_state      = self.get_pipeline_state(&pipeline_config);

        let mut render_state    = RenderState {
            main_buffer:            target_drawable,
            target_buffer:          target_drawable.clone(),
            matrix:                 matrix,
            pipeline_config:        pipeline_config,
            pipeline_state:         pipeline_state,
            command_buffer:         command_queue.new_command_buffer()
        };

        // Evaluate the actions
        for action in actions {
            use self::RenderAction::*;

            match action {
                SetTransform(matrix)                                                    => { self.set_transform(matrix, &mut render_state); }
                CreateVertex2DBuffer(id, vertices)                                      => { self.create_vertex_buffer_2d(id, vertices); }
                CreateIndexBuffer(id, indices)                                          => { self.create_index_buffer(id, indices); }
                FreeVertexBuffer(id)                                                    => { self.free_vertex_buffer(id); }
                FreeIndexBuffer(id)                                                     => { self.free_index_buffer(id); }
                BlendMode(blend_mode)                                                   => { self.blend_mode(blend_mode, &mut render_state); }
                CreateRenderTarget(render_id, texture_id, width, height, render_type)   => { self.create_render_target(render_id, texture_id, width, height, render_type); }
                FreeRenderTarget(render_id)                                             => { self.free_render_target(render_id); }
                SelectRenderTarget(render_id)                                           => { self.select_render_target(render_id); }
                RenderToFrameBuffer                                                     => { self.select_main_frame_buffer(&mut render_state); }
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

    ///
    /// Sets the active transformation matrix
    ///
    fn set_transform(&mut self, matrix: Matrix, state: &mut RenderState) {
        state.matrix.set_matrix(matrix);
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

    ///
    /// Releases the memory associated with a vertex buffer
    ///
    fn free_vertex_buffer(&mut self, VertexBufferId(vertex_id): VertexBufferId) {
        self.vertex_buffers[vertex_id] = None;
    }

    ///
    /// Frees the index buffer with the specified ID
    ///
    fn free_index_buffer(&mut self, IndexBufferId(id): IndexBufferId) {
        self.index_buffers[id] = None;
    }

    ///
    /// Updates the blend mode for a render state
    ///
    fn blend_mode(&mut self, blend_mode: BlendMode, state: &mut RenderState) {
        state.pipeline_config.blend_mode    = blend_mode;
        state.pipeline_state                = self.get_pipeline_state(&state.pipeline_config);
    }

    fn create_render_target(&mut self, RenderTargetId(render_id): RenderTargetId, TextureId(texture_id): TextureId, width: usize, height: usize, render_target_type: RenderTargetType) {

    }

    fn free_render_target(&mut self, RenderTargetId(render_id): RenderTargetId) {

    }

    fn select_render_target(&mut self, RenderTargetId(render_id): RenderTargetId) {

    }

    ///
    /// Sets the main frame buffer to be the current render target
    ///
    fn select_main_frame_buffer(&mut self, state: &mut RenderState) {
        state.target_buffer = state.main_buffer.clone();
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
