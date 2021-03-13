use super::error::*;
use super::buffer::*;
use super::texture::*;
use super::vertex_array::*;
use super::render_target::*;
use super::shader_uniforms::*;
use super::shader_collection::*;

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

    /// The shader that's currently set to be used
    active_shader: Option<ShaderType>,

    /// The matrix that's currently in use
    transform_matrix: Option<[gl::types::GLfloat; 16]>,

    /// The 'main' render target that represents the output for this renderer
    default_render_target: Option<RenderTarget>,

    /// The render targets assigned to this renderer
    render_targets: Vec<Option<RenderTarget>>,

    /// The simple shader program
    simple_shader: ShaderCollection<ShaderUniform>,

    /// The 'dashed line' shader program
    dashed_line_shader: ShaderCollection<ShaderUniform>,

    /// The 'texture fill' shader program
    texture_shader: ShaderCollection<ShaderUniform>
}

impl GlRenderer {
    ///
    /// Creates a new renderer that will render to the specified device and factory
    ///
    pub fn new() -> GlRenderer {
        let simple_vertex                       = String::from_utf8(include_bytes!["../../shaders/simple/simple.glslv"].to_vec()).unwrap();
        let simple_fragment                     = String::from_utf8(include_bytes!["../../shaders/simple/simple.glslf"].to_vec()).unwrap();
        let dashed_line_fragment                = String::from_utf8(include_bytes!["../../shaders/dashed_line/dashed_line.glslf"].to_vec()).unwrap();
        let texture_vertex                      = String::from_utf8(include_bytes!["../../shaders/texture/texture.glslv"].to_vec()).unwrap();
        let texture_fragment                    = String::from_utf8(include_bytes!["../../shaders/texture/texture.glslf"].to_vec()).unwrap();

        let simple_shader                       = ShaderCollection::new(&simple_vertex, vec!["a_Pos", "a_Color", "a_TexCoord"], &simple_fragment, vec![]);
        let dashed_line_shader                  = ShaderCollection::new(&simple_vertex, vec!["a_Pos", "a_Color", "a_TexCoord"], &dashed_line_fragment, vec![]);
        let texture_shader                      = ShaderCollection::new(&texture_vertex, vec!["a_Pos", "a_Color", "a_TexCoord"], &texture_fragment, vec![]);

        GlRenderer {
            buffers:                        vec![],
            index_buffers:                  vec![],
            textures:                       vec![],
            default_render_target:          None,
            active_shader:                  None,
            transform_matrix:               None,
            render_targets:                 vec![],
            simple_shader:                  simple_shader,
            dashed_line_shader:             dashed_line_shader,
            texture_shader:                 texture_shader
        }
    }

    ///
    /// Prepares to render to the active framebuffer
    ///
    pub fn prepare_to_render_to_active_framebuffer(&mut self, width: usize, height: usize) {
        unsafe {
            panic_on_gl_error("Preparing to render");

            // Set the default render target to be a reference to the current render target
            self.default_render_target = Some(RenderTarget::reference_to_current());

            // Set the viewport to the specified width and height
            gl::Viewport(0, 0, width as gl::types::GLsizei, height as gl::types::GLsizei);

            self.active_shader      = None;
            self.transform_matrix   = Some(Matrix::identity().to_opengl_matrix());

            panic_on_gl_error("After preparing to render");
        }
    }

    ///
    /// Performs rendering of the specified actions to this device target
    ///
    pub fn render<Actions: IntoIterator<Item=RenderAction>>(&mut self, actions: Actions) {
        // Enable options
        self.enable_options();

        panic_on_gl_error("Enabling options");

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
                CreateTextureMono(texture_id, width, height)                            => { self.create_mono_texture(texture_id, width, height); }
                Create1DTextureBgra(texture_id, width)                                  => { self.create_1d_bgra_texture(texture_id, width); }
                Create1DTextureMono(texture_id, width)                                  => { self.create_1d_mono_texture(texture_id, width); }
                WriteTextureData(texture_id, (x1, y1), (x2, y2), data)                  => { self.write_texture_data_2d(texture_id, (x1, y1), (x2, y2), &*data); }
                WriteTexture1D(texture_id, x1, x2, data)                                => { self.write_texture_data_1d(texture_id, x1, x2, &*data); }
                CreateMipMaps(texture_id)                                               => { self.create_mipmaps(texture_id); }
                CopyTexture(source, target)                                             => { self.copy_texture(source, target); }
                FreeTexture(texture_id)                                                 => { self.free_texture(texture_id); }
                Clear(color)                                                            => { self.clear(color); }
                UseShader(shader_type)                                                  => { self.use_shader(shader_type); }
                DrawTriangles(buffer_id, buffer_range)                                  => { self.draw_triangles(buffer_id, buffer_range); }
                DrawIndexedTriangles(vertex_buffer, index_buffer, num_vertices)         => { self.draw_indexed_triangles(vertex_buffer, index_buffer, num_vertices); }
            }

            panic_on_gl_error("Post-action");
        }

        // Reset options
        self.disable_options();

        panic_on_gl_error("Render tidy up");
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
            gl::UseProgram(*self.simple_shader.basic);

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
    /// Creates a new monochrome texture
    ///
    fn create_mono_texture(&mut self, TextureId(texture_id): TextureId, width: usize, height: usize) {
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
        new_texture.create_monochrome(width as u16, height as u16);

        // Store the texture
        self.textures[texture_id] = Some(new_texture);
    }
    
    ///
    /// Creates a 1 dimensional BGRA texture
    ///
    /// (This is useful for things like describing gradiant fill patterns)
    ///
    fn create_1d_bgra_texture(&mut self, TextureId(texture_id): TextureId, width: usize) {
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
        new_texture.create_empty_1d(width as u16);

        // Store the texture
        self.textures[texture_id] = Some(new_texture);
    }
    
    ///
    /// Creates a 1 dimensional mono texture
    ///
    /// (This is useful for things like describing dash patterns)
    ///
    fn create_1d_mono_texture(&mut self, TextureId(texture_id): TextureId, width: usize) {
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
        new_texture.create_monochrome_1d(width as u16);

        // Store the texture
        self.textures[texture_id] = Some(new_texture);
    }
    
    ///
    /// Writes out byte data to a region in a 2D texture
    ///
    fn write_texture_data_2d(&mut self, TextureId(texture_id): TextureId, (x1, y1): (usize, usize), (x2, y2): (usize, usize), data: &[u8]) {
        if let Some(Some(texture)) = self.textures.get_mut(texture_id) {
            if texture.is_mono() {
                texture.set_data_mono(x1, y1, x2-x1, y2-y1, data);
            } else {
                texture.set_data_rgba(x1, y1, x2-x1, y2-y1, data);
            }
        }
    }
    
    ///
    /// Writes out byte data to a region in a 1D texture
    ///
    fn write_texture_data_1d(&mut self, TextureId(texture_id): TextureId, x1: usize, x2: usize, data: &[u8]) {
        if let Some(Some(texture)) = self.textures.get_mut(texture_id) {
            if texture.is_mono() {
                texture.set_data_mono_1d(x1, x2-x1, data);
            } else {
                texture.set_data_rgba_1d(x1, x2-x1, data);
            }
        }
    }
    
    ///
    /// Generates mip-maps for a texture to prepare it for rendering
    ///
    fn create_mipmaps(&mut self, TextureId(texture_id): TextureId) {
        if texture_id < self.textures.len() {
            // Mip-map the texture if it exists in this renderer
            self.textures[texture_id].as_mut().map(|texture| texture.generate_mipmaps());
        }
    }

    fn copy_texture(&mut self, TextureId(source_id): TextureId, TextureId(target_id): TextureId) {
        // Extend the textures array as needed
        if source_id >= self.textures.len() {
            self.textures.extend((self.textures.len()..(source_id+1))
                .into_iter()
                .map(|_| None));
        }

        if target_id >= self.textures.len() {
            self.textures.extend((self.textures.len()..(target_id+1))
                .into_iter()
                .map(|_| None));
        }

        // Free the target texture first
        self.textures[target_id] = None;

        if source_id < self.textures.len() {
            // Ask the source to copy itself
            self.textures[target_id] = self.textures[source_id].as_ref().and_then(|texture| texture.make_copy());
        }
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
                // TODO: to get the background colour to show up properly, need to draw using the frame buffer texture
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
        use self::ShaderType::*;

        self.active_shader = Some(shader_type);

        match shader_type {
            Simple { erase_texture, clip_texture } => {
                let simple_shader   = &mut self.simple_shader;
                let textures        = &self.textures;
                let erase_texture   = erase_texture.and_then(|TextureId(texture_id)| textures[texture_id].as_ref());
                let clip_texture    = clip_texture.and_then(|TextureId(texture_id)| textures[texture_id].as_ref());

                simple_shader.use_shader(ShaderUniform::EraseTexture, ShaderUniform::ClipTexture, erase_texture, clip_texture);

                panic_on_gl_error("Set simple shader");
            }

            DashedLine { dash_texture, erase_texture, clip_texture } => {
                // Set the basic clip/erase textures
                let dash_shader             = &mut self.dashed_line_shader;
                let textures                = &self.textures;
                let TextureId(dash_texture) = dash_texture;
                let dash_texture            = self.textures[dash_texture].as_ref();
                let erase_texture           = erase_texture.and_then(|TextureId(texture_id)| textures[texture_id].as_ref());
                let clip_texture            = clip_texture.and_then(|TextureId(texture_id)| textures[texture_id].as_ref());

                let program                 = dash_shader.use_shader(ShaderUniform::EraseTexture, ShaderUniform::ClipTexture, erase_texture, clip_texture);

                // Set the dash texture
                if let Some(dash_texture) = dash_texture {
                    unsafe {
                        gl::ActiveTexture(gl::TEXTURE0);
                        gl::BindTexture(gl::TEXTURE_1D, **dash_texture);

                        program.uniform_location(ShaderUniform::DashTexture, "t_DashPattern")
                            .map(|dash_pattern| {
                                gl::Uniform1i(dash_pattern, 0);
                            });
                    }
                } else {
                    // Texture not found: revert to the simple shader
                    self.simple_shader.use_shader(ShaderUniform::EraseTexture, ShaderUniform::ClipTexture, None, None);
                }

                panic_on_gl_error("Set dash shader");
            }

            Texture { texture, texture_transform, repeat, erase_texture, clip_texture } => {
                let texture_shader      = &mut self.texture_shader;
                let textures            = &self.textures;
                let TextureId(texture)  = texture;
                let texture             = if texture < self.textures.len() { self.textures[texture].as_ref() } else { None };
                let erase_texture       = erase_texture.and_then(|TextureId(texture_id)| textures[texture_id].as_ref());
                let clip_texture        = clip_texture.and_then(|TextureId(texture_id)| textures[texture_id].as_ref());
                let texture_transform   = texture_transform.to_opengl_matrix();

                let program             = texture_shader.use_shader(ShaderUniform::EraseTexture, ShaderUniform::ClipTexture, erase_texture, clip_texture);

                // Set up the texture program
                if let Some(texture) = texture {
                    unsafe {
                        // Bind the texture to texture 0
                        gl::ActiveTexture(gl::TEXTURE0);
                        gl::BindTexture(gl::TEXTURE_2D, **texture);

                        if repeat {
                            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as _);
                            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as _);
                        } else {
                            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as _);
                            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as _);
                        }

                        // Set in the program uniform
                        program.uniform_location(ShaderUniform::Texture, "t_Texture")
                            .map(|texture_uniform| {
                                gl::Uniform1i(texture_uniform, 0);
                            });
                        program.uniform_location(ShaderUniform::TextureTransform, "texture_transform")
                            .map(|transform_uniform| {
                                gl::UniformMatrix4fv(transform_uniform, 1, gl::FALSE, texture_transform.as_ptr());
                            });
                    }
                } else {
                    // Texture not found: revert to the simple shader
                    self.simple_shader.use_shader(ShaderUniform::EraseTexture, ShaderUniform::ClipTexture, None, None);
                }

                panic_on_gl_error("Set texture shader");
            }
        }

        // Set the transform for the newly selected shader
        self.update_shader_transform();
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
            if vertex_buffer >= self.buffers.len() || index_buffer >= self.index_buffers.len() {
                // Treat the same as the buffer being none
                // TODO: this seems to happen sometimes with the flo_draw examples on Windows, but it's not clear why and is likely a bug
                // It's possible we're rendering things out of order somehow (eg, rendering the results of a 'resize' event after the initial draw, so the buffers aren't loaded)
                return;
            }

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
        self.transform_matrix = Some(matrix.to_opengl_matrix());

        // Store in the uniform in all of the shaders
        self.update_shader_transform();
    }

    ///
    /// Set the transform in the active shader
    ///
    fn update_shader_transform(&mut self) {
        unsafe {
            use self::ShaderType::*;

            let shader = match &self.active_shader {
                Some(Simple { erase_texture: None, clip_texture: None })                            => Some(&mut self.simple_shader.basic),
                Some(Simple { erase_texture: Some(_), clip_texture: None })                         => Some(&mut self.simple_shader.erase),
                Some(Simple { erase_texture: None, clip_texture: Some(_) })                         => Some(&mut self.simple_shader.clip),
                Some(Simple { erase_texture: Some(_), clip_texture: Some(_) })                      => Some(&mut self.simple_shader.clip_erase),

                Some(DashedLine { dash_texture: _, erase_texture: None, clip_texture: None })       => Some(&mut self.dashed_line_shader.basic),
                Some(DashedLine { dash_texture: _, erase_texture: Some(_), clip_texture: None })    => Some(&mut self.dashed_line_shader.erase),
                Some(DashedLine { dash_texture: _, erase_texture: None, clip_texture: Some(_) })    => Some(&mut self.dashed_line_shader.clip),
                Some(DashedLine { dash_texture: _, erase_texture: Some(_), clip_texture: Some(_) }) => Some(&mut self.dashed_line_shader.clip_erase),

                Some(Texture { erase_texture: None, clip_texture: None, .. })                       => Some(&mut self.texture_shader.basic),
                Some(Texture { erase_texture: Some(_), clip_texture: None, .. })                    => Some(&mut self.texture_shader.erase),
                Some(Texture { erase_texture: None, clip_texture: Some(_), .. })                    => Some(&mut self.texture_shader.clip),
                Some(Texture { erase_texture: Some(_), clip_texture: Some(_), .. })                 => Some(&mut self.texture_shader.clip_erase),

                None                                                                                => None
            };

            self.transform_matrix.as_ref().and_then(|transform_matrix|
                shader.map(|shader| (shader, transform_matrix))
            ).map(|(shader, matrix)| {
                shader.uniform_location(ShaderUniform::Transform, "transform").map(|transform_uniform| {
                    gl::UniformMatrix4fv(transform_uniform, 1, gl::FALSE, matrix.as_ptr());
                });
            });
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
