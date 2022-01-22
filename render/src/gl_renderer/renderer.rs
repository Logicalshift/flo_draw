use super::error::*;
use super::buffer::*;
use super::texture::*;
use super::vertex_array::*;
use super::render_target::*;
use super::shader_uniforms::*;
use super::shader_collection::*;
use super::standard_shader_programs::*;

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

    /// The shader programs
    shader_programs: ShaderCollection<StandardShaderProgram, ShaderUniform>,
}

impl GlRenderer {
    ///
    /// Creates a new renderer that will render to the specified device and factory
    ///
    pub fn new() -> GlRenderer {
        let shader_programs     = ShaderCollection::new(StandardShaderProgram::create_shader_loader());

        GlRenderer {
            buffers:                        vec![],
            index_buffers:                  vec![],
            textures:                       vec![],
            default_render_target:          None,
            active_shader:                  None,
            transform_matrix:               None,
            render_targets:                 vec![],
            shader_programs:                shader_programs,
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
                SetTransform(matrix)                                                            => { self.set_transform(matrix); }
                CreateVertex2DBuffer(id, vertices)                                              => { self.create_vertex_buffer_2d(id, vertices); }
                CreateIndexBuffer(id, indices)                                                  => { self.create_index_buffer(id, indices); }
                FreeVertexBuffer(id)                                                            => { self.free_vertex_buffer(id); }
                FreeIndexBuffer(id)                                                             => { self.free_index_buffer(id); }
                BlendMode(blend_mode)                                                           => { self.blend_mode(blend_mode); }
                CreateRenderTarget(render_id, texture_id, Size2D(width, height), render_type)   => { self.create_render_target(render_id, texture_id, width, height, render_type); }
                FreeRenderTarget(render_id)                                                     => { self.free_render_target(render_id); }
                SelectRenderTarget(render_id)                                                   => { self.select_render_target(render_id); }
                RenderToFrameBuffer                                                             => { self.select_main_frame_buffer(); }
                DrawFrameBuffer(render_id, region, Alpha(alpha))                                => { self.draw_frame_buffer(render_id, region, alpha); }
                ShowFrameBuffer                                                                 => { /* This doesn't double-buffer so nothing to do */ }
                CreateTextureBgra(texture_id, Size2D(width, height))                            => { self.create_bgra_texture(texture_id, width, height); }
                CreateTextureMono(texture_id, Size2D(width, height))                            => { self.create_mono_texture(texture_id, width, height); }
                Create1DTextureBgra(texture_id, Size1D(width))                                  => { self.create_1d_bgra_texture(texture_id, width); }
                Create1DTextureMono(texture_id, Size1D(width))                                  => { self.create_1d_mono_texture(texture_id, width); }
                WriteTextureData(texture_id, Position2D(x1, y1), Position2D(x2, y2), data)      => { self.write_texture_data_2d(texture_id, (x1, y1), (x2, y2), &*data); }
                WriteTexture1D(texture_id, Position1D(x1), Position1D(x2), data)                => { self.write_texture_data_1d(texture_id, x1, x2, &*data); }
                CreateMipMaps(texture_id)                                                       => { self.create_mipmaps(texture_id); }
                CopyTexture(source, target)                                                     => { self.copy_texture(source, target); }
                FreeTexture(texture_id)                                                         => { self.free_texture(texture_id); }
                Clear(color)                                                                    => { self.clear(color); }
                UseShader(shader_type)                                                          => { self.use_shader(shader_type); }
                DrawTriangles(buffer_id, buffer_range)                                          => { self.draw_triangles(buffer_id, buffer_range); }
                DrawIndexedTriangles(vertex_buffer, index_buffer, num_vertices)                 => { self.draw_indexed_triangles(vertex_buffer, index_buffer, num_vertices); }
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
            self.shader_programs.use_program(StandardShaderProgram::Simple(StandardShaderVariant::NoClipping));

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

                // TODO: these both require the shader to multiply the alpha in (a modification to the texture shader)
                Multiply            => gl::BlendFuncSeparate(gl::DST_COLOR, gl::ONE_MINUS_SRC_ALPHA, gl::ZERO, gl::ONE),
                Screen              => gl::BlendFuncSeparate(gl::ONE_MINUS_DST_COLOR, gl::ONE, gl::ZERO, gl::ONE),

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
                let (width, height) = render_target.get_size();

                gl::BindFramebuffer(gl::FRAMEBUFFER, **render_target);
                gl::Viewport(0, 0, width as gl::types::GLsizei, height as gl::types::GLsizei);
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
    fn draw_frame_buffer(&mut self, RenderTargetId(source_buffer): RenderTargetId, region: FrameBufferRegion, alpha: f64) {
        let shaders         = &mut self.shader_programs;

        self.render_targets[source_buffer].as_ref().map(|source_buffer| {
            unsafe {
                if let Some(texture) = source_buffer.texture() {
                    // Activate the resolving program
                    let shader = shaders.use_program(StandardShaderProgram::MsaaResolve(4));

                    // Set the texture for the render buffer
                    gl::ActiveTexture(gl::TEXTURE0);
                    gl::BindTexture(gl::TEXTURE_2D_MULTISAMPLE, *texture);

                    shader.uniform_location(ShaderUniform::MsaaAlpha, "t_Alpha")
                        .map(|t_alpha| {
                            gl::Uniform1f(t_alpha, alpha as _);
                        });

                    shader.uniform_location(ShaderUniform::MsaaTexture, "t_SourceTexture")
                        .map(|source_texture| {
                            gl::Uniform1i(source_texture, 0);
                        });

                    // Create the vertices for the two triangles making up the screen
                    let min_x               = region.min_x();
                    let min_y               = region.min_y();
                    let max_x               = region.max_x();
                    let max_y               = region.max_y();

                    let vertices            = vec![
                        Vertex2D::with_pos(min_x, min_y), Vertex2D::with_pos(max_x, min_y), Vertex2D::with_pos(min_x, max_y),
                        Vertex2D::with_pos(max_x, min_y), Vertex2D::with_pos(min_x, max_y), Vertex2D::with_pos(max_x, max_y),
                    ];
                    let mut buffer          = Buffer::new();
                    let vertex_array        = VertexArray::new();
                    buffer.static_draw(&vertices);

                    // Bind a vertex array object to it
                    gl::BindVertexArray(*vertex_array);
                    gl::BindBuffer(gl::ARRAY_BUFFER, *buffer);

                    Vertex2D::define_attributes();

                    // Clear the bindings
                    gl::BindBuffer(gl::ARRAY_BUFFER, 0);
                    gl::BindVertexArray(0);

                    // Render a quad filling the screen
                    gl::BindVertexArray(*vertex_array);
                    gl::DrawArrays(gl::TRIANGLES, 0, 6);

                    gl::BindVertexArray(0);
                } else {
                    // Blit the framebuffer if we're using a renderbuffer directly instead of a backing texture (won't blend or obey the alpha value)
                    let (x, y)          = (0, 0);
                    let (width, height) = source_buffer.get_size();
                    let width           = width as i32;
                    let height          = height as i32;

                    gl::BindFramebuffer(gl::READ_FRAMEBUFFER, **source_buffer);
                    gl::BlitFramebuffer(0, 0, width, height, x, y, x+width, y+height, gl::COLOR_BUFFER_BIT, gl::NEAREST);
                }

                // We always revert to the simple shader after this operation
                shaders.use_program(StandardShaderProgram::Simple(StandardShaderVariant::NoClipping));

                // Finish up by checking for errors
                panic_on_gl_error("Draw frame buffer");
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
    /// Returns the shader program identifier to use for the currently selected shader
    ///
    fn active_shader_program(&self) -> Option<StandardShaderProgram> {
        use self::ShaderType::*;

        match &self.active_shader {
            Some(Simple { clip_texture: None })                                     => Some(StandardShaderProgram::Simple(StandardShaderVariant::NoClipping)),
            Some(Simple { clip_texture: Some(_) })                                  => Some(StandardShaderProgram::Simple(StandardShaderVariant::ClippingMask)),

            Some(DashedLine { dash_texture: _, clip_texture: None })                => Some(StandardShaderProgram::DashedLine(StandardShaderVariant::NoClipping)),
            Some(DashedLine { dash_texture: _, clip_texture: Some(_) })             => Some(StandardShaderProgram::DashedLine(StandardShaderVariant::ClippingMask)),

            Some(Texture { clip_texture: None, premultiply_colours: false, .. })    => Some(StandardShaderProgram::Texture(StandardShaderVariant::NoClipping, ColorPostProcessingStep::NoPostProcessing)),
            Some(Texture { clip_texture: Some(_), premultiply_colours: false, .. }) => Some(StandardShaderProgram::Texture(StandardShaderVariant::ClippingMask, ColorPostProcessingStep::NoPostProcessing)),
            Some(Texture { clip_texture: None, premultiply_colours: true, .. })     => Some(StandardShaderProgram::Texture(StandardShaderVariant::NoClipping, ColorPostProcessingStep::MultiplyAlpha)),
            Some(Texture { clip_texture: Some(_), premultiply_colours: true, .. })  => Some(StandardShaderProgram::Texture(StandardShaderVariant::ClippingMask, ColorPostProcessingStep::MultiplyAlpha)),

            Some(LinearGradient { clip_texture: None, .. })                         => Some(StandardShaderProgram::LinearGradient(StandardShaderVariant::NoClipping)),
            Some(LinearGradient { clip_texture: Some(_), .. })                      => Some(StandardShaderProgram::LinearGradient(StandardShaderVariant::ClippingMask)),

            None                                                                    => None
        }
    }

    ///
    /// Enables a particular shader for future rendering operations
    ///
    fn use_shader(&mut self, shader_type: ShaderType) {
        use self::ShaderType::*;

        self.active_shader = Some(shader_type);

        match shader_type {
            Simple { clip_texture } => {
                let textures        = &self.textures;
                let clip_texture    = clip_texture.and_then(|TextureId(texture_id)| textures[texture_id].as_ref());
                let variant         = if clip_texture.is_some() { StandardShaderVariant::ClippingMask } else { StandardShaderVariant::NoClipping };

                let program = self.shader_programs.use_program(StandardShaderProgram::Simple(variant));
                if let Some(clip_texture) = clip_texture { program.use_texture(ShaderUniform::ClipTexture, "t_ClipMask", clip_texture, 2); }

                panic_on_gl_error("Set simple shader");
            }

            DashedLine { dash_texture, clip_texture } => {
                // Set the basic clip/erase textures
                let textures                = &self.textures;
                let TextureId(dash_texture) = dash_texture;
                let dash_texture            = self.textures[dash_texture].as_ref();
                let clip_texture            = clip_texture.and_then(|TextureId(texture_id)| textures[texture_id].as_ref());
                let variant                 = if clip_texture.is_some() { StandardShaderVariant::ClippingMask } else { StandardShaderVariant::NoClipping };

                let program                 = self.shader_programs.use_program(StandardShaderProgram::DashedLine(variant));
                if let Some(clip_texture) = clip_texture { program.use_texture(ShaderUniform::ClipTexture, "t_ClipMask", clip_texture, 2); }

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
                    self.shader_programs.use_program(StandardShaderProgram::default());
                }

                panic_on_gl_error("Set dash shader");
            }

            Texture { texture, texture_transform, repeat, alpha, premultiply_colours, clip_texture } => {
                let textures            = &self.textures;
                let TextureId(texture)  = texture;
                let texture             = if texture < self.textures.len() { self.textures[texture].as_ref() } else { None };
                let clip_texture        = clip_texture.and_then(|TextureId(texture_id)| textures[texture_id].as_ref());
                let variant             = if clip_texture.is_some() { StandardShaderVariant::ClippingMask } else { StandardShaderVariant::NoClipping };
                let premultiply         = if premultiply_colours { ColorPostProcessingStep::MultiplyAlpha } else { ColorPostProcessingStep::NoPostProcessing };
                let texture_transform   = texture_transform.to_opengl_matrix();

                let program             = self.shader_programs.use_program(StandardShaderProgram::Texture(variant, premultiply));
                if let Some(clip_texture) = clip_texture { program.use_texture(ShaderUniform::ClipTexture, "t_ClipMask", clip_texture, 2); }

                // Set up the texture program
                if let Some(texture) = texture {
                    unsafe {
                        // Bind the texture to texture 0
                        gl::ActiveTexture(gl::TEXTURE0);
                        gl::BindTexture(gl::TEXTURE_2D, **texture);

                        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR_MIPMAP_LINEAR as _);
                        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as _);

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
                        program.uniform_location(ShaderUniform::TextureAlpha, "texture_alpha")
                            .map(|alpha_uniform| {
                                gl::Uniform1f(alpha_uniform, alpha);
                            });
                    }
                } else {
                    // Texture not found: revert to the simple shader
                    self.shader_programs.use_program(StandardShaderProgram::default());
                }

                panic_on_gl_error("Set texture shader");
            }

            LinearGradient { texture, texture_transform, repeat, alpha, clip_texture } => {
                let textures            = &self.textures;
                let TextureId(texture)  = texture;
                let texture             = if texture < self.textures.len() { self.textures[texture].as_ref() } else { None };
                let clip_texture        = clip_texture.and_then(|TextureId(texture_id)| textures[texture_id].as_ref());
                let variant             = if clip_texture.is_some() { StandardShaderVariant::ClippingMask } else { StandardShaderVariant::NoClipping };
                let texture_transform   = texture_transform.to_opengl_matrix();

                let program             = self.shader_programs.use_program(StandardShaderProgram::LinearGradient(variant));
                if let Some(clip_texture) = clip_texture { program.use_texture(ShaderUniform::ClipTexture, "t_ClipMask", clip_texture, 2); }

                // Set up the texture program
                if let Some(texture) = texture {
                    unsafe {
                        // Bind the texture to texture 0
                        gl::ActiveTexture(gl::TEXTURE0);
                        gl::BindTexture(gl::TEXTURE_1D, **texture);

                        gl::TexParameteri(gl::TEXTURE_1D, gl::TEXTURE_MIN_FILTER, gl::LINEAR_MIPMAP_LINEAR as _);
                        gl::TexParameteri(gl::TEXTURE_1D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as _);

                        if repeat {
                            gl::TexParameteri(gl::TEXTURE_1D, gl::TEXTURE_WRAP_S, gl::REPEAT as _);
                            gl::TexParameteri(gl::TEXTURE_1D, gl::TEXTURE_WRAP_T, gl::REPEAT as _);
                        } else {
                            gl::TexParameteri(gl::TEXTURE_1D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as _);
                            gl::TexParameteri(gl::TEXTURE_1D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as _);
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
                        program.uniform_location(ShaderUniform::TextureAlpha, "texture_alpha")
                            .map(|alpha_uniform| {
                                gl::Uniform1f(alpha_uniform, alpha);
                            });
                    }
                } else {
                    // Texture not found: revert to the simple shader
                    self.shader_programs.use_program(StandardShaderProgram::default());
                }

                panic_on_gl_error("Set linear gradient shader");
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
            let shader              = self.active_shader_program();
            let shader_programs     = &mut self.shader_programs;
            let transform_matrix    = &self.transform_matrix;
            let shader              = shader.map(|shader| shader_programs.program(shader));

            transform_matrix.as_ref().and_then(|transform_matrix|
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
