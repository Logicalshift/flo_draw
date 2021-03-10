use super::error::*;

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
    texture: Rc<TextureRef>,

    texture_target: gl::types::GLuint,
    texture_format: gl::types::GLuint,
    width:          gl::types::GLsizei,
    height:         gl::types::GLsizei,
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
                texture_target: gl::TEXTURE_2D,
                texture_format: gl::RGBA,
                width:          0,
                height:         0
            }
        }
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
    /// Sets 8-bit BGRA pixel data for a texture
    ///
    pub fn set_data_bgra(&mut self, x: usize, y: usize, width: usize, height: usize, pixels: &[u8]) {
        if pixels.len() != (width * height * 4) {
            panic!("set_data_bgra called with incorrect sized pixel array")
        }

        unsafe {
            gl::BindTexture(self.texture_target, self.texture.texture_id);
            gl::TexSubImage2D(gl::TEXTURE_2D, 0, x as _, y as _, width as _, height as _, gl::BGRA, gl::UNSIGNED_BYTE, pixels.as_ptr() as _);

            panic_on_gl_error("Set bgra data");
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
    /// Sets 8-bit BGRA pixel data for a texture
    ///
    pub fn set_data_bgra_1d(&mut self, x: usize, width: usize, pixels: &[u8]) {
        if pixels.len() != width * 4 {
            panic!("set_data_bgra_1d called with incorrect sized pixel array")
        }

        unsafe {
            gl::BindTexture(self.texture_target, self.texture.texture_id);
            gl::TexSubImage1D(gl::TEXTURE_1D, 0, x as _, width as _, gl::BGRA, gl::UNSIGNED_BYTE, pixels.as_ptr() as _);

            panic_on_gl_error("Set bgra 1D data");
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
    /// Creates a copy of this texture, if possible
    ///
    pub fn make_copy(&self) -> Option<Texture> {
        // Allocate a new texture for the copy
        let mut copy    = Texture::new();
        let texture_id  = copy.texture.texture_id;

        // Fetch information on the existing texture
        let format      = self.texture_format;
        let width       = self.width;
        let height      = self.height;

        // TODO: attach the existing texture to the read buffer

        // Generate the main texture image
        match self.texture_target {
            gl::TEXTURE_1D => {
                gl::BindTexture(gl::TEXTURE_1D, texture_id);

                gl::TexParameteri(gl::TEXTURE_1D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
                gl::TexParameteri(gl::TEXTURE_1D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);

                gl::TexImage1D(gl::TEXTURE_1D, 0, format as _, width, 0, format, gl::UNSIGNED_BYTE, ptr::null());
            }

            gl::TEXTURE_2D => {
                gl::BindTexture(gl::TEXTURE_2D, texture_id);

                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);

                gl::TexImage2D(gl::TEXTURE_2D, 0, format as _, width, height, 0, format, gl::UNSIGNED_BYTE, ptr::null());
            }

            gl::TEXTURE_2D_MULTISAMPLE => {
                // Clamp the number of samples to the maximum supported by the driver
                let mut max_samples = 1;
                gl::GetIntegerv(gl::MAX_COLOR_TEXTURE_SAMPLES, &mut max_samples);
                let samples = max_samples.min(samples as i32);

                // Set up a MSAA texture
                gl::BindTexture(gl::TEXTURE_2D_MULTISAMPLE, texture_id);

                gl::TexImage2DMultisample(gl::TEXTURE_2D_MULTISAMPLE, samples, gl::RGBA, width as _, height as _, gl::FALSE);
            }

            _ => { 
                // Don't know how to copy this target type
                return None;
            }
        }

        unimplemented!()
    }

    ///
    /// True if this is a monochrome texture
    ///
    pub fn is_mono(&self) -> bool {
        self.texture_format == gl::RED
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
