use super::buffer::*;
use super::shader::*;
use super::texture::*;
use super::vertex_array::*;
use super::render_target::*;
use super::shader_program::*;
use super::shader_uniforms::*;

use crate::action::*;
use crate::buffer::*;

use std::ptr;
use std::ops::{Range};

///
/// OpenGL action renderer
///
pub struct GlRenderer {
    /// The buffers allocated to this renderer and their corresponding vertex array object
    buffers: Vec<Option<(VertexArray, Buffer)>>,

    /// The index buffers defined for this renderer
    index_buffers: Vec<Option<Buffer>>,

    /// The textures allocated to this renderer
    textures: Vec<Option<Texture>>,

    /// The 'main' render target that represents the output for this renderer
    default_render_target: Option<RenderTarget>,

    /// The render targets assigned to this renderer
    render_targets: Vec<Option<RenderTarget>>,

    /// The simple shader program
    simple_shader: ShaderProgram<ShaderUniform>,

    /// The shader program that applies an erase buffer
    simple_shader_with_erase: ShaderProgram<ShaderUniform>
}

impl GlRenderer {
    ///
    /// Creates a new renderer that will render to the specified device and factory
    ///
    pub fn new() -> GlRenderer {
        let simple_vertex_shader            = Shader::compile(&String::from_utf8(include_bytes!["../../shaders/simple/simple.glslv"].to_vec()).unwrap(), GlShaderType::Vertex, vec!["a_Pos", "a_Color", "a_TexCoord"]);
        let simple_fragment_shader          = Shader::compile(&(String::from("#version 330 core\n") + &String::from_utf8(include_bytes!["../../shaders/simple/simple.glslf"].to_vec()).unwrap()), GlShaderType::Fragment, vec![]);
        let simple_shader                   = ShaderProgram::from_shaders(vec![simple_vertex_shader, simple_fragment_shader]);

        let simple_vertex_shader            = Shader::compile(&String::from_utf8(include_bytes!["../../shaders/simple/simple.glslv"].to_vec()).unwrap(), GlShaderType::Vertex, vec!["a_Pos", "a_Color", "a_TexCoord"]);
        let simple_erase_fragment_shader    = Shader::compile(&(String::from("#version 330 core\n#define ERASE_MASK\n") + &String::from_utf8(include_bytes!["../../shaders/simple/simple.glslf"].to_vec()).unwrap()), GlShaderType::Fragment, vec![]);
        let simple_shader_with_erase        = ShaderProgram::from_shaders(vec![simple_vertex_shader, simple_erase_fragment_shader]);

        GlRenderer {
            buffers:                    vec![],
            index_buffers:              vec![],
            textures:                   vec![],
            default_render_target:      None,
            render_targets:             vec![],
            simple_shader:              simple_shader,
            simple_shader_with_erase:   simple_shader_with_erase
        }
    }

    ///
    /// Prepares to render to the active framebuffer
    ///
    pub fn prepare_to_render_to_active_framebuffer(&mut self, width: usize, height: usize) {
        unsafe {
            // Set the default render target to be a reference to the current render target
            self.default_render_target = Some(RenderTarget::reference_to_current());

            // Set the viewport to the specified width and height
            gl::Viewport(0, 0, width as gl::types::GLsizei, height as gl::types::GLsizei);

            self.set_transform(Matrix::identity());
        }
    }

    ///
    /// Performs rendering of the specified actions to this device target
    ///
    pub fn render<Actions: IntoIterator<Item=RenderAction>>(&mut self, actions: Actions) {
        // Enable options
        self.enable_options();

        for action in actions {
            use self::RenderAction::*;

            match action {
                SetTransform(matrix)                                                    => { self.set_transform(matrix); }
                CreateVertex2DBuffer(id, vertices)                                      => { self.create_vertex_buffer_2d(id, vertices); }
                CreateIndexBuffer(id, indices)                                          => { self.create_index_buffer(id, indices); }
                FreeVertexBuffer(id)                                                    => { self.free_vertex_buffer(id); }
                FreeIndexBuffer(id)                                                     => { self.free_index_buffer(id); }
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

        // Reset options
        self.disable_options();
    }

    ///
    /// Sets the GL options that apply across all operations for this renderer
    ///
    fn enable_options(&mut self) {
        unsafe {
            // Turn on blending
            gl::Enable(gl::BLEND);
            gl::BlendEquationSeparate(gl::FUNC_ADD, gl::FUNC_ADD);

            // Use the basic shader program by default
            gl::UseProgram(*self.simple_shader);

            self.blend_mode(BlendMode::SourceOver);
        }
    }

    ///
    /// Disables the GL options enabled by enable_options
    ///
    fn disable_options(&self) {
        unsafe {
            gl::Disable(gl::BLEND);
        }
    }

    ///
    /// Clears the current render target
    ///
    fn clear(&mut self, Rgba8([r, g, b, a]): Rgba8) {
        let r = (r as f32)/255.0;
        let g = (g as f32)/255.0;
        let b = (b as f32)/255.0;
        let a = (a as f32)/255.0;

        unsafe { 
            // Clear the buffer
            gl::ClearBufferfv(gl::COLOR, 0, &[r, g, b, a][0]); 
        }
    }

    ///
    /// Creates a 2D vertex buffer
    ///
    fn create_vertex_buffer_2d(&mut self, VertexBufferId(buffer_id): VertexBufferId, vertices: Vec<Vertex2D>) {
        // Extend the buffers array as needed
        if buffer_id >= self.buffers.len() {
            self.buffers.extend((self.buffers.len()..(buffer_id+1))
                .into_iter()
                .map(|_| None));
        }

        // Release the previous buffer
        self.buffers[buffer_id] = None;

        // Create a buffer containing these vertices
        let mut buffer          = Buffer::new();
        let vertex_array        = VertexArray::new();
        buffer.static_draw(&vertices);

        unsafe {
            // Bind a vertex array object to it
            gl::BindVertexArray(*vertex_array);
            gl::BindBuffer(gl::ARRAY_BUFFER, *buffer);

            Vertex2D::define_attributes();

            // Clear the bindings
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindVertexArray(0);
        }

        // Store in the buffers collections
        self.buffers[buffer_id] = Some((vertex_array, buffer));
    }

    ///
    /// Creates an index buffer
    ///
    fn create_index_buffer(&mut self, IndexBufferId(buffer_id): IndexBufferId, indices: Vec<u16>) {
        // Extend the buffers array as needed
        if buffer_id >= self.index_buffers.len() {
            self.index_buffers.extend((self.index_buffers.len()..(buffer_id+1))
                .into_iter()
                .map(|_| None));
        }

        // Release the previous buffer
        self.index_buffers[buffer_id] = None;

        // Create a buffer containing these indices
        let mut buffer          = Buffer::new();
        buffer.static_draw(&indices);

        // Store in the buffers collections
        self.index_buffers[buffer_id] = Some(buffer);
    }

    ///
    /// Frees the vertex buffer with the specified ID
    ///
    fn free_vertex_buffer(&mut self, VertexBufferId(id): VertexBufferId) {
        self.buffers[id] = None;
    }

    ///
    /// Frees the index buffer with the specified ID
    ///
    fn free_index_buffer(&mut self, IndexBufferId(id): IndexBufferId) {
        self.index_buffers[id] = None;
    }

    ///
    /// Sets the blending mode to use
    ///
    fn blend_mode(&mut self, blend_mode: BlendMode) {
        use self::BlendMode::*;

        unsafe {
            match blend_mode {
                SourceOver          => gl::BlendFuncSeparate(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA, gl::ONE, gl::ONE_MINUS_SRC_ALPHA),
                DestinationOver     => gl::BlendFuncSeparate(gl::ONE_MINUS_DST_ALPHA, gl::DST_ALPHA, gl::ONE_MINUS_DST_ALPHA, gl::ONE),
                SourceIn            => gl::BlendFuncSeparate(gl::DST_ALPHA, gl::ZERO, gl::DST_ALPHA, gl::ZERO),
                DestinationIn       => gl::BlendFuncSeparate(gl::ZERO, gl::SRC_ALPHA, gl::ZERO, gl::SRC_ALPHA),
                SourceOut           => gl::BlendFuncSeparate(gl::ZERO, gl::ONE_MINUS_DST_ALPHA, gl::ZERO, gl::ONE_MINUS_DST_ALPHA),
                DestinationOut      => gl::BlendFuncSeparate(gl::ZERO, gl::ONE_MINUS_SRC_ALPHA, gl::ZERO, gl::ONE_MINUS_SRC_ALPHA),
                SourceATop          => gl::BlendFuncSeparate(gl::ONE_MINUS_DST_ALPHA, gl::SRC_ALPHA, gl::ONE_MINUS_DST_ALPHA, gl::SRC_ALPHA),
                DestinationATop     => gl::BlendFuncSeparate(gl::ONE_MINUS_DST_ALPHA, gl::ONE_MINUS_SRC_ALPHA, gl::ONE_MINUS_DST_ALPHA, gl::ONE_MINUS_SRC_ALPHA),

                AllChannelAlphaSourceOver       => gl::BlendFuncSeparate(gl::ONE, gl::ONE_MINUS_SRC_COLOR, gl::ONE, gl::ONE_MINUS_SRC_ALPHA),
                AllChannelAlphaDestinationOver  => gl::BlendFuncSeparate(gl::ONE_MINUS_DST_COLOR, gl::ONE, gl::ONE_MINUS_DST_ALPHA, gl::ONE),
            }
        }
    }

    ///
    /// Creates a new BGRA texture
    ///
    fn create_bgra_texture(&mut self, TextureId(texture_id): TextureId, width: usize, height: usize) {
        // Extend the textures array as needed
        if texture_id >= self.textures.len() {
            self.textures.extend((self.textures.len()..(texture_id+1))
                .into_iter()
                .map(|_| None));
        }

        // Free any existing texture
        self.textures[texture_id] = None;

        // Create a new texture
        let mut new_texture = Texture::new();
        new_texture.create_empty(width as u16, height as u16);

        // Store the texture
        self.textures[texture_id] = Some(new_texture);
    }

    ///
    /// Releases an existing render target
    ///
    fn free_texture(&mut self, TextureId(texture_id): TextureId) {
        self.textures[texture_id] = None;
    }

    ///
    /// Creates a new render target
    ///
    fn create_render_target(&mut self, RenderTargetId(render_id): RenderTargetId, TextureId(texture_id): TextureId, width: usize, height: usize, render_type: RenderTargetType) {
        // Extend the textures array as needed
        if texture_id >= self.textures.len() {
            self.textures.extend((self.textures.len()..(texture_id+1))
                .into_iter()
                .map(|_| None));
        }

        // Extend the render targets array as needed
        if render_id >= self.render_targets.len() {
            self.render_targets.extend((self.render_targets.len()..(render_id+1))
                .into_iter()
                .map(|_| None));
        }

        // Free any existing texture and render target
        self.textures[texture_id]       = None;
        self.render_targets[render_id]  = None;

        // Create the new render target
        let new_render_target           = RenderTarget::new(width as u16, height as u16, render_type);

        // Store the properties of the new render target
        self.textures[texture_id]       = new_render_target.texture();
        self.render_targets[render_id]  = Some(new_render_target);
    }

    ///
    /// Chooses which buffer rendering instructions will be sent to
    ///
    fn select_render_target(&mut self, RenderTargetId(render_id): RenderTargetId) {
        self.render_targets[render_id].as_ref().map(|render_target| {
            unsafe {
                gl::BindFramebuffer(gl::FRAMEBUFFER, **render_target)
            }
        });
    }

    ///
    /// Sends rendering instructions to the primary frame buffer for display
    ///
    fn select_main_frame_buffer(&mut self) {
        self.default_render_target.as_ref().map(|render_target| {
            unsafe {
                gl::BindFramebuffer(gl::FRAMEBUFFER, **render_target)
            }
        });
    }

    ///
    /// Draws a frame buffer at a location
    ///
    fn draw_frame_buffer(&mut self, RenderTargetId(source_buffer): RenderTargetId, x: i32, y: i32) {
        self.render_targets[source_buffer].as_ref().map(|source_buffer| {
            unsafe {
                let (width, height) = source_buffer.get_size();
                let width           = width as i32;
                let height          = height as i32;

                gl::BindFramebuffer(gl::READ_FRAMEBUFFER, **source_buffer);
                gl::BlitFramebuffer(0, 0, width, height, x, y, x+width, y+height, gl::COLOR_BUFFER_BIT, gl::NEAREST);
            }
        });
    }

    ///
    /// Releases an existing render target
    ///
    fn free_render_target(&mut self, RenderTargetId(render_id): RenderTargetId) {
        self.render_targets[render_id] = None;
    }

    ///
    /// Enables a particular shader for future rendering operations
    ///
    fn use_shader(&mut self, shader_type: ShaderType) {
        unsafe {
            use self::ShaderType::*;

            match shader_type {
                Simple { erase_texture: None } => { gl::UseProgram(*self.simple_shader); }

                Simple { erase_texture: Some(TextureId(texture_id)) } => { 
                    gl::UseProgram(*self.simple_shader_with_erase);

                    if let Some(texture) = &self.textures[texture_id] {
                        // Set the erase texture
                        gl::ActiveTexture(gl::TEXTURE0);
                        gl::BindTexture(gl::TEXTURE_2D_MULTISAMPLE, **texture);

                        self.simple_shader_with_erase.uniform_location(ShaderUniform::EraseTexture, "t_EraseMask")
                            .map(|erase_mask| {
                                gl::Uniform1i(erase_mask, 0);
                            });
                    }

                }
            }
        }
    }

    ///
    /// Draw triangles from a buffer
    ///
    fn draw_triangles(&mut self, VertexBufferId(buffer_id): VertexBufferId, buffer_range: Range<usize>) {
        unsafe {
            if let Some((vertex_array, _buffer)) = &self.buffers[buffer_id] {
                // Draw the triangles
                gl::BindVertexArray(**vertex_array);
                gl::DrawArrays(gl::TRIANGLES, buffer_range.start as gl::types::GLint, buffer_range.len() as gl::types::GLsizei);

                gl::BindVertexArray(0);
            }
        }
    }

    ///
    /// Draw triangles from a buffer
    ///
    fn draw_indexed_triangles(&mut self, VertexBufferId(vertex_buffer): VertexBufferId, IndexBufferId(index_buffer): IndexBufferId, num_vertices: usize) {
        unsafe {
            if let (Some((vertex_array, _buffer)), Some(index_buffer)) = (&self.buffers[vertex_buffer], &self.index_buffers[index_buffer]) {
                let num_vertices = num_vertices as gl::types::GLsizei;

                // Draw the triangles
                gl::BindVertexArray(**vertex_array);
                gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, **index_buffer);
                gl::DrawElements(gl::TRIANGLES, num_vertices, gl::UNSIGNED_SHORT, ptr::null());

                gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, 0);
                gl::BindVertexArray(0);
            }
        }
    }

    ///
    /// Sets the transformation matrix for this renderer
    ///
    fn set_transform(&mut self, matrix: Matrix) {
        // Convert to an OpenGL matrix
        let matrix: [gl::types::GLfloat; 16] = matrix.to_opengl_matrix();

        // Store in the uniform in all of the shaders
        unsafe {
            for shader in vec![&mut self.simple_shader, &mut self.simple_shader_with_erase].into_iter() {
                shader.uniform_location(ShaderUniform::Transform, "transform").map(|transform_uniform| {
                    gl::UniformMatrix4fv(transform_uniform, 1, gl::FALSE, matrix.as_ptr());
                });
            }
        }
    }

    ///
    /// Flushes all changes to the device
    ///
    pub fn flush(&mut self) {
        unsafe {
            gl::Flush();
        }
    }
}
