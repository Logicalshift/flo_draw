use super::error::*;
use super::buffer::*;
use super::vertex_array::*;
use super::render_target::*;
use super::shader_program::*;
use super::shader_uniforms::*;

use crate::buffer::*;

use gl;

use std::ptr;
use std::rc::*;
use std::ops::{Deref};

struct TextureRef {
    texture_id: gl::types::GLuint,
}

///
/// Abstraction that manages an OpenGL texture
///
#[derive(Clone)]
pub struct Texture {
    texture:                    Rc<TextureRef>,

    pub (super) premultiplied:  bool,
    pub (super) texture_target: gl::types::GLuint,
    pub (super) texture_format: gl::types::GLuint,
    num_samples:                usize, 
    pub (super) width:          gl::types::GLsizei,
    pub (super) height:         gl::types::GLsizei,
}

impl Texture {
    ///
    /// Creates a new OpenGL texture object
    ///
    pub fn new() -> Texture {
        unsafe {
            let mut new_texture = 0;
            gl::GenTextures(1, &mut new_texture);

            Texture {
                texture:        Rc::new(TextureRef { texture_id: new_texture }),
                premultiplied:  false,
                texture_target: gl::TEXTURE_2D,
                texture_format: gl::RGBA,
                num_samples:    0,
                width:          0,
                height:         0
            }
        }
    }

    ///
    /// Creates an empty texture that's the equivalent of the specified texture
    ///
    pub fn empty_equivalent(properties_from: &Texture) -> Option<Texture> {
        // Create a new texture with the same properties as the existing one
        let mut new_texture = Self::new();

        match (properties_from.texture_target, properties_from.texture_format) {
            (gl::TEXTURE_2D, gl::RGBA)              => new_texture.create_empty(properties_from.width as _, properties_from.height as _),
            (gl::TEXTURE_2D, gl::RED)               => new_texture.create_monochrome(properties_from.width as _, properties_from.height as _),
            (gl::TEXTURE_2D_MULTISAMPLE, gl::RGBA)  => new_texture.create_empty_multisampled(properties_from.width as _, properties_from.height as _, properties_from.num_samples as _),
            (gl::TEXTURE_2D_MULTISAMPLE, gl::RED)   => new_texture.create_monochrome_multisampled(properties_from.width as _, properties_from.height as _, properties_from.num_samples as _),

            (gl::TEXTURE_1D, gl::RGBA)              => new_texture.create_empty_1d(properties_from.width as _),
            (gl::TEXTURE_1D, gl::RED)               => new_texture.create_monochrome_1d(properties_from.width as _),

            _                                       => { return None; }
        };

        // Copy over other properties
        new_texture.premultiplied = properties_from.premultiplied;

        Some(new_texture)
    }

    ///
    /// Associates an empty image with this texture
    ///
    pub fn create_empty(&mut self, width: u16, height: u16) {
        unsafe {
            let texture_id      = self.texture.texture_id;
            self.texture_target = gl::TEXTURE_2D;
            self.texture_format = gl::RGBA;
            self.width          = width as _;
            self.height         = height as _;

            gl::BindTexture(gl::TEXTURE_2D, texture_id);

            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);

            gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RGBA as _, width as _, height as _, 0, gl::RGBA, gl::UNSIGNED_BYTE, ptr::null());

            panic_on_gl_error("Create texture");
        }
    }

    ///
    /// Creates an empty MSAA texture
    ///
    pub fn create_empty_multisampled(&mut self, width: u16, height: u16, samples: usize) {
        unsafe {
            let texture_id      = self.texture.texture_id;
            self.texture_target = gl::TEXTURE_2D_MULTISAMPLE;
            self.texture_format = gl::RGBA;
            self.width          = width as _;
            self.height         = height as _;

            // Clamp the number of samples to the maximum supported by the driver
            let mut max_samples = 1;
            gl::GetIntegerv(gl::MAX_COLOR_TEXTURE_SAMPLES, &mut max_samples);
            let samples = max_samples.min(samples as i32);

            self.num_samples    = samples as _;

            // Set up a MSAA texture
            gl::BindTexture(gl::TEXTURE_2D_MULTISAMPLE, texture_id);

            gl::TexImage2DMultisample(gl::TEXTURE_2D_MULTISAMPLE, samples, gl::RGBA, width as _, height as _, gl::FALSE);

            panic_on_gl_error("Create multisampled texture");
        }
    }

    ///
    /// Associates an empty image with this texture
    ///
    pub fn create_monochrome(&mut self, width: u16, height: u16) {
        unsafe {
            let texture_id      = self.texture.texture_id;
            self.texture_target = gl::TEXTURE_2D;
            self.texture_format = gl::RED;
            self.width          = width as _;
            self.height         = height as _;

            gl::BindTexture(gl::TEXTURE_2D, texture_id);

            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);

            gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RED as _, width as _, height as _, 0, gl::RED, gl::UNSIGNED_BYTE, ptr::null());

            panic_on_gl_error("Create monochrome texture");
        }
    }

    ///
    /// Creates an empty MSAA texture
    ///
    pub fn create_monochrome_multisampled(&mut self, width: u16, height: u16, samples: usize) {
        unsafe {
            let texture_id      = self.texture.texture_id;
            self.texture_target = gl::TEXTURE_2D_MULTISAMPLE;
            self.texture_format = gl::RED;
            self.width          = width as _;
            self.height         = height as _;

            // Clamp the number of samples to the maximum supported by the driver
            let mut max_samples = 1;
            gl::GetIntegerv(gl::MAX_COLOR_TEXTURE_SAMPLES, &mut max_samples);
            let samples = max_samples.min(samples as i32);

            self.num_samples    = samples as _;

            // Set up a MSAA texture
            gl::BindTexture(gl::TEXTURE_2D_MULTISAMPLE, texture_id);

            gl::TexImage2DMultisample(gl::TEXTURE_2D_MULTISAMPLE, samples, gl::RED, width as _, height as _, gl::FALSE);

            panic_on_gl_error("Create monochrome multisampled texture");
        }
    }

    ///
    /// Associates an empty image with this texture
    ///
    pub fn create_empty_1d(&mut self, width: u16) {
        unsafe {
            let texture_id      = self.texture.texture_id;
            self.texture_target = gl::TEXTURE_1D;
            self.texture_format = gl::RGBA;
            self.width          = width as _;
            self.height         = 1;

            gl::BindTexture(gl::TEXTURE_1D, texture_id);

            gl::TexParameteri(gl::TEXTURE_1D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_1D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);

            gl::TexImage1D(gl::TEXTURE_1D, 0, gl::RGBA as _, width as _, 0, gl::RGBA, gl::UNSIGNED_BYTE, ptr::null());

            panic_on_gl_error("Create 1D texture");
        }
    }

    ///
    /// Associates an empty image with this texture
    ///
    pub fn create_monochrome_1d(&mut self, width: u16) {
        unsafe {
            let texture_id      = self.texture.texture_id;
            self.texture_target = gl::TEXTURE_1D;
            self.texture_format = gl::RED;
            self.width          = width as _;
            self.height         = 1;

            gl::BindTexture(gl::TEXTURE_1D, texture_id);

            gl::TexParameteri(gl::TEXTURE_1D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_1D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);

            gl::TexImage1D(gl::TEXTURE_1D, 0, gl::RED as _, width as _, 0, gl::RED, gl::UNSIGNED_BYTE, ptr::null());

            panic_on_gl_error("Create 1D mono texture");
        }
    }

    ///
    /// Associates an empty image with this texture
    ///
    pub fn create_monochrome_1d_float(&mut self, width: u16) {
        unsafe {
            let texture_id      = self.texture.texture_id;
            self.texture_target = gl::TEXTURE_1D;
            self.texture_format = gl::R16F;
            self.width          = width as _;
            self.height         = 1;

            gl::BindTexture(gl::TEXTURE_1D, texture_id);

            gl::TexParameteri(gl::TEXTURE_1D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_1D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);

            gl::TexImage1D(gl::TEXTURE_1D, 0, gl::R16F as _, width as _, 0, gl::RED, gl::FLOAT, ptr::null());

            panic_on_gl_error("Create 1D mono texture (floating point)");
        }
    }

    ///
    /// Generates mip-maps for this texture
    ///
    pub fn generate_mipmaps(&mut self) {
        unsafe {
            gl::BindTexture(self.texture_target, self.texture.texture_id);
            gl::GenerateMipmap(self.texture_target);

            panic_on_gl_error("Generate texture mip map");
        }
    }

    ///
    /// Sets 8-bit RGBA pixel data for a texture
    ///
    pub fn set_data_rgba(&mut self, x: usize, y: usize, width: usize, height: usize, pixels: &[u8]) {
        if pixels.len() != (width * height * 4) {
            panic!("set_data_bgra called with incorrect sized pixel array")
        }

        unsafe {
            gl::BindTexture(self.texture_target, self.texture.texture_id);
            gl::TexSubImage2D(gl::TEXTURE_2D, 0, x as _, y as _, width as _, height as _, gl::RGBA, gl::UNSIGNED_BYTE, pixels.as_ptr() as _);

            panic_on_gl_error("Set rgba data");
        }
    }

    ///
    /// Sets 8-bit mono pixel data for a texture
    ///
    pub fn set_data_mono(&mut self, x: usize, y: usize, width: usize, height: usize, pixels: &[u8]) {
        if pixels.len() != width * height {
            panic!("set_data_mono called with incorrect sized pixel array")
        }

        unsafe {
            gl::BindTexture(self.texture_target, self.texture.texture_id);
            gl::TexSubImage2D(gl::TEXTURE_2D, 0, x as _, y as _, width as _, height as _, gl::RED, gl::UNSIGNED_BYTE, pixels.as_ptr() as _);

            panic_on_gl_error("Set mono data");
        }
    }

    ///
    /// Sets 8-bit RGBA pixel data for a texture
    ///
    pub fn set_data_rgba_1d(&mut self, x: usize, width: usize, pixels: &[u8]) {
        if pixels.len() != width * 4 {
            panic!("set_data_bgra_1d called with incorrect sized pixel array")
        }

        unsafe {
            gl::BindTexture(self.texture_target, self.texture.texture_id);
            gl::TexSubImage1D(gl::TEXTURE_1D, 0, x as _, width as _, gl::RGBA, gl::UNSIGNED_BYTE, pixels.as_ptr() as _);

            panic_on_gl_error("Set rgba 1D data");
        }
    }

    ///
    /// Sets 8-bit mono pixel data for a texture
    ///
    pub fn set_data_mono_1d(&mut self, x: usize, width: usize, pixels: &[u8]) {
        if pixels.len() != width {
            panic!("set_data_mono_1d called with incorrect sized pixel array")
        }

        unsafe {
            gl::BindTexture(self.texture_target, self.texture.texture_id);
            gl::TexSubImage1D(gl::TEXTURE_1D, 0, x as _, width as _, gl::RED, gl::UNSIGNED_BYTE, pixels.as_ptr() as _);

            panic_on_gl_error("Set mono 1D data");
        }
    }

    ///
    /// Sets 8-bit mono pixel data for a texture
    ///
    pub fn set_data_mono_1d_float(&mut self, x: usize, width: usize, pixels: &[f32]) {
        if pixels.len() != width {
            panic!("set_data_mono_1d_f16 called with incorrect sized pixel array")
        }

        unsafe {
            gl::BindTexture(self.texture_target, self.texture.texture_id);
            gl::TexSubImage1D(gl::TEXTURE_1D, 0, x as _, width as _, gl::RED, gl::FLOAT, pixels.as_ptr() as _);

            panic_on_gl_error("Set mono 1D data (float)");
        }
    }

    ///
    /// Creates a copy of this texture, if possible
    ///
    pub fn make_copy(&self) -> Option<Texture> {
        unsafe {
            // Allocate a new texture for the copy
            let mut copy            = Texture::new();
            let texture_id          = copy.texture.texture_id;

            // Fetch information on the existing texture
            let format              = self.texture_format;
            let width               = self.width;
            let height              = self.height;

            // Give the copy the same texture properties as the original
            copy.texture_format     = format;
            copy.width              = width;
            copy.height             = height;
            copy.premultiplied      = self.premultiplied;

            // Attach the existing texture to the read buffer
            let existing_texture    = RenderTarget::from_texture(self)?;

            // Generate the main texture image
            match self.texture_target {
                gl::TEXTURE_1D => {
                    gl::BindTexture(gl::TEXTURE_1D, texture_id);

                    gl::TexParameteri(gl::TEXTURE_1D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
                    gl::TexParameteri(gl::TEXTURE_1D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);

                    gl::TexImage1D(gl::TEXTURE_1D, 0, format as _, width, 0, format, gl::UNSIGNED_BYTE, ptr::null());
                    panic_on_gl_error("Create 1D copy target");
                }

                gl::TEXTURE_2D => {
                    gl::BindTexture(gl::TEXTURE_2D, texture_id);

                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);

                    gl::TexImage2D(gl::TEXTURE_2D, 0, format as _, width, height, 0, format, gl::UNSIGNED_BYTE, ptr::null());
                    panic_on_gl_error("Create 2D copy target");
                }

                gl::TEXTURE_2D_MULTISAMPLE => {
                    // Clamp the number of samples to the maximum supported by the driver
                    let mut max_samples = 1;
                    let samples         = self.num_samples;
                    gl::GetIntegerv(gl::MAX_COLOR_TEXTURE_SAMPLES, &mut max_samples);
                    let samples = max_samples.min(samples as i32);

                    // Set up a MSAA texture
                    gl::BindTexture(gl::TEXTURE_2D_MULTISAMPLE, texture_id);

                    gl::TexImage2DMultisample(gl::TEXTURE_2D_MULTISAMPLE, samples, gl::RGBA, width as _, height as _, gl::FALSE);
                    panic_on_gl_error("Create multisampled copy target");
                }

                _ => { 
                    // Don't know how to copy this target type
                    return None;
                }
            }


            // Find the currently bound frame buffer (so we can rebind it later on)
            let mut old_frame_buffer = 0;
            gl::GetIntegerv(gl::DRAW_FRAMEBUFFER_BINDING, &mut old_frame_buffer);
            panic_on_gl_error("Get old framebuffer");

            // Bind to the frame buffer
            gl::BindFramebuffer(gl::FRAMEBUFFER, *existing_texture);
            gl::ReadBuffer(gl::COLOR_ATTACHMENT0);
            panic_on_gl_error("Bind new framebuffer");

            // Copy the first subimage to the new texture from the old one
            match self.texture_target {
                gl::TEXTURE_1D => {
                    gl::CopyTexImage1D(gl::TEXTURE_1D, 0, format, 0, 0, width, 0);
                    panic_on_gl_error("Copy 1D");
                }

                gl::TEXTURE_2D => {
                    gl::CopyTexImage2D(gl::TEXTURE_2D, 0, format, 0, 0, width, height, 0);
                    panic_on_gl_error("Copy 2D");
                }

                _ => { /* Don't know how to copy */ }
            }

            // Bind back to the old framebuffer
            gl::BindFramebuffer(gl::FRAMEBUFFER, old_frame_buffer as _);

            // Return the copied texture
            Some(copy)
        }
    }

    ///
    /// True if this is a monochrome texture
    ///
    pub fn is_mono(&self) -> bool {
        self.texture_format == gl::RED
    }

    ///
    /// Performs a filter operation, creating a new texture (typically used to replace this one)
    ///
    /// This sets up the new texture as a render target, sets the rendering state for filtering and then performs
    /// the filter operation using the currently selected texture
    ///
    pub fn filter<'a>(&self, filter_shader: &'a mut ShaderProgram<ShaderUniform>) -> Option<Texture> {
        unsafe {
            // Create a texture blank that's equivalent of this one
            let new_texture = Self::empty_equivalent(self)?;

            // Activate the texture and set up the rendering parameters
            gl::UseProgram(**filter_shader);
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(self.texture_target, **self);

            gl::Disable(gl::BLEND);

            // Bilinear filtering (useful for things like gaussian blur)
            gl::TexParameteri(self.texture_target, gl::TEXTURE_MIN_FILTER, gl::LINEAR as _);
            gl::TexParameteri(self.texture_target, gl::TEXTURE_MAG_FILTER, gl::LINEAR as _);

            // Texture wrap is clamp to edge
            gl::TexParameteri(self.texture_target, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as _);
            gl::TexParameteri(self.texture_target, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as _);

            // Set the current texture in the shader program
            filter_shader.uniform_location(ShaderUniform::Texture, "t_Texture")
                .map(|texture_uniform| {
                    gl::Uniform1i(texture_uniform, 0);
                });

            // Create a render target for the new texture
            let original_render_target  = RenderTarget::reference_to_current(0, 0);
            let filter_render_target    = RenderTarget::from_texture(&new_texture)?;

            let mut original_viewport: [gl::types::GLint; 4] = [0, 0, 0, 0];
            gl::GetIntegerv(gl::VIEWPORT, &mut original_viewport[0]);

            panic_on_gl_error("Set up for texture filtering");

            // Bind to the render target
            gl::BindFramebuffer(gl::FRAMEBUFFER, *filter_render_target);
            gl::Viewport(0, 0, new_texture.width as _, new_texture.height as _);

            // Create some vertices representing the triangles the fill the render target
            let vertices        = vec![
                Vertex2D::with_pos(-1.0, -1.0), Vertex2D::with_pos(1.0, -1.0), Vertex2D::with_pos(-1.0, 1.0),
                Vertex2D::with_pos(1.0, -1.0), Vertex2D::with_pos(-1.0, 1.0), Vertex2D::with_pos(1.0, 1.0),
            ];
            let mut buffer      = Buffer::new();
            let vertex_array    = VertexArray::new();
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

            // Reset to the original render target
            gl::BindFramebuffer(gl::FRAMEBUFFER, *original_render_target);
            gl::Viewport(original_viewport[0], original_viewport[1], original_viewport[2], original_viewport[3]);

            gl::Enable(gl::BLEND);

            panic_on_gl_error("Texture filter");

            // The new texture is the result
            Some(new_texture)
        }
    }
}

impl Drop for TextureRef {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteTextures(1, &mut self.texture_id);
        }
    }
}

impl Deref for Texture {
    type Target = gl::types::GLuint;

    fn deref(&self) -> &gl::types::GLuint {
        &self.texture.texture_id
    }
}
