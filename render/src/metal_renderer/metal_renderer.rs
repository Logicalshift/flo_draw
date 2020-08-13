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

    /// The render targets for this renderer
    render_targets: Vec<Option<metal::Texture>>,

    /// The tetures for this renderer
    textures: Vec<Option<metal::Texture>>,

    /// The cache of render pipeline states used by this renderer
    pipeline_states: HashMap<PipelineConfiguration, metal::RenderPipelineState>
}

///
/// The current state of a renderer
///
struct RenderState<'a> {
    /// The main render buffer
    main_buffer: &'a metal::Drawable,

    /// The main render buffer texture
    main_texture: metal::Texture,

    /// The current target render buffer
    target_texture: metal::Texture,

    /// Buffer containing the current transformation matrix
    matrix: MatrixBuffer,

    /// The active pipeline configuration
    pipeline_config: PipelineConfiguration,

    /// The active pipeline state corresponding to the pipeline configuration
    pipeline_state: metal::RenderPipelineState,

    /// The command buffer we're using to send rendering actions
    command_buffer: &'a metal::CommandBufferRef,

    /// The command encoder we're currently writing to
    command_encoder: &'a metal::RenderCommandEncoderRef
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
            index_buffers:      vec![],
            render_targets:     vec![],
            textures:           vec![],
            shader_library:     shader_library,
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
    /// Creates a command encoder for rendering to the specified texture
    ///
    fn get_command_encoder<'a>(&mut self, command_buffer: &'a metal::CommandBufferRef, render_target: &metal::Texture) -> &'a metal::RenderCommandEncoderRef {
        let render_descriptor   = metal::RenderPassDescriptor::new();
        let color_attachment    = render_descriptor.color_attachments().object_at(0).unwrap();

        color_attachment.set_texture(Some(render_target));
        color_attachment.set_load_action(metal::MTLLoadAction::Load);
        color_attachment.set_store_action(metal::MTLStoreAction::Store);

        command_buffer.new_render_command_encoder(&render_descriptor)
    }

    ///
    /// Performs rendering of the specified actions to this device target
    ///
    pub fn render<Actions: IntoIterator<Item=RenderAction>>(&mut self, actions: Actions, target_drawable: &metal::Drawable, target_texture: &metal::Texture) {
        // Create the render state
        let command_queue       = self.command_queue.clone();
        let matrix              = MatrixBuffer::from_matrix(&self.device, Matrix::identity());
        let pipeline_config     = PipelineConfiguration::default();
        let pipeline_state      = self.get_pipeline_state(&pipeline_config);
        let command_buffer      = command_queue.new_command_buffer();
        let command_encoder     = self.get_command_encoder(command_buffer, target_texture);

        command_encoder.set_render_pipeline_state(&pipeline_state);

        let mut render_state    = RenderState {
            main_buffer:            target_drawable,
            main_texture:           target_texture.clone(),
            target_texture:         target_texture.clone(),
            matrix:                 matrix,
            pipeline_config:        pipeline_config,
            pipeline_state:         pipeline_state,
            command_buffer:         command_buffer,
            command_encoder:        command_encoder
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
                SelectRenderTarget(render_id)                                           => { self.select_render_target(render_id, &mut render_state); }
                RenderToFrameBuffer                                                     => { self.select_main_frame_buffer(&mut render_state); }
                DrawFrameBuffer(render_id, x, y)                                        => { self.draw_frame_buffer(render_id, x, y); }
                ShowFrameBuffer                                                         => { /* This doesn't double-buffer so nothing to do */ }
                CreateTextureBgra(texture_id, width, height)                            => { self.create_bgra_texture(texture_id, width, height); }
                FreeTexture(texture_id)                                                 => { self.free_texture(texture_id); }
                Clear(color)                                                            => { self.clear(color); }
                UseShader(shader_type)                                                  => { self.use_shader(shader_type); }
                DrawTriangles(buffer_id, buffer_range)                                  => { self.draw_triangles(buffer_id, buffer_range, &mut render_state); }
                DrawIndexedTriangles(vertex_buffer, index_buffer, num_vertices)         => { self.draw_indexed_triangles(vertex_buffer, index_buffer, num_vertices, &mut render_state); }
            }
        }

        // Finish up
        render_state.command_encoder.end_encoding();
        command_buffer.present_drawable(target_drawable);
        command_buffer.commit();
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
        state.command_encoder.set_render_pipeline_state(&state.pipeline_state);
    }

    ///
    /// Creates a render target and its backing texture
    ///
    fn create_render_target(&mut self, RenderTargetId(render_id): RenderTargetId, TextureId(texture_id): TextureId, width: usize, height: usize, render_target_type: RenderTargetType) {
        // Allocate space for the texture and render target
        if render_id >= self.render_targets.len() {
            self.render_targets.extend((self.render_targets.len()..(render_id+1))
                .into_iter()
                .map(|_| None));
        }

        if texture_id >= self.textures.len() {
            self.textures.extend((self.textures.len()..(texture_id+1))
                .into_iter()
                .map(|_| None));
        }

        // Free any existing texture or render target
        self.render_targets[render_id]  = None;
        self.textures[texture_id]       = None;

        // Create the texture descriptor
        let texture_descriptor = metal::TextureDescriptor::new();

        texture_descriptor.set_width(width as u64);
        texture_descriptor.set_height(height as u64);
        texture_descriptor.set_pixel_format(metal::MTLPixelFormat::RGBA8Unorm);
        texture_descriptor.set_usage(metal::MTLTextureUsage::RenderTarget);

        match render_target_type {
            RenderTargetType::Standard              => { }

            RenderTargetType::Multisampled          |
            RenderTargetType::MultisampledTexture   => { 
                texture_descriptor.set_sample_count(4);
            }

            RenderTargetType::Monochrome            => {
                texture_descriptor.set_pixel_format(metal::MTLPixelFormat::R8Unorm);
            }

            RenderTargetType::MonochromeMultisampledTexture => {
                texture_descriptor.set_pixel_format(metal::MTLPixelFormat::R8Unorm);
                texture_descriptor.set_sample_count(4);
            }
        }

        // Turn into a texture
        let render_texture              = self.device.new_texture(&texture_descriptor);

        self.render_targets[render_id]  = Some(render_texture.clone());
        self.textures[texture_id]       = Some(render_texture.clone());
    }

    ///
    /// Frees up a render target for this renderer
    ///
    fn free_render_target(&mut self, RenderTargetId(render_id): RenderTargetId) {
        self.render_targets[render_id] = None;
    }

    ///
    /// Selects an alternative render target
    ///
    fn select_render_target(&mut self, RenderTargetId(render_id): RenderTargetId, state: &mut RenderState) {
        // Fetch the render texture
        let texture = match &self.render_targets[render_id] { Some(texture) => texture, None => { return } };

        // Set the state to point at the new texture
        state.target_texture = texture.clone();

        // Create a command encoder that will use this texture
        state.command_encoder.end_encoding();
        state.command_encoder = self.get_command_encoder(state.command_buffer, &state.target_texture);
    }

    ///
    /// Sets the main frame buffer to be the current render target
    ///
    fn select_main_frame_buffer(&mut self, state: &mut RenderState) {
        // Reset the state to point at the main texture
        state.target_texture = state.main_texture.clone();

        // Create a command encoder that will use this texture
        state.command_encoder.end_encoding();
        state.command_encoder = self.get_command_encoder(state.command_buffer, &state.target_texture);
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

    ///
    /// Draws triangles from a vertex buffer
    ///
    fn draw_triangles(&mut self, VertexBufferId(vertex_buffer_id): VertexBufferId, range: Range<usize>, state: &mut RenderState) {
        // Fetch the buffer to draw
        let buffer = match &self.vertex_buffers[vertex_buffer_id] { Some(buffer) => buffer, None => { return } };

        // Draw these vertices
        state.command_encoder.set_vertex_buffer(0, Some(buffer), 0);
        state.command_encoder.draw_primitives(metal::MTLPrimitiveType::Triangle, range.start as u64, range.len() as u64);
    }

    ///
    /// Draws triangles using vertices referenced by an index buffer
    ///
    fn draw_indexed_triangles(&mut self, VertexBufferId(vertex_buffer_id): VertexBufferId, IndexBufferId(index_buffer_id): IndexBufferId, num_vertices: usize, state: &mut RenderState) {
        // Fetch the buffer and index buffer to draw
        let vertex_buffer   = match &self.vertex_buffers[vertex_buffer_id] { Some(buffer) => buffer, None => { return } };
        let index_buffer    = match &self.index_buffers[index_buffer_id] { Some(buffer) => buffer, None => { return } };

        // Draw these vertices
        state.command_encoder.set_vertex_buffer(0, Some(vertex_buffer), 0);
        state.command_encoder.draw_indexed_primitives(metal::MTLPrimitiveType::Triangle, num_vertices as u64, metal::MTLIndexType::UInt16, index_buffer, 0);
    }
}
