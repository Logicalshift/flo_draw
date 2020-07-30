use gl;

use std::ptr;
use std::rc::*;
use std::ops::{Deref};

struct TextureRef {
    texture_id: gl::types::GLuint
}

///
/// Abstraction that manages an OpenGL texture
///
#[derive(Clone)]
pub struct Texture {
    texture: Rc<TextureRef>
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
                texture: Rc::new(TextureRef { texture_id: new_texture })
            }
        }
    }

    ///
    /// Associates an empty image with this texture
    ///
    pub fn create_empty(&mut self, width: u16, height: u16) {
        unsafe {
            let texture_id = self.texture.texture_id;

            gl::BindTexture(gl::TEXTURE_2D, texture_id);

            gl::TextureParameteri(texture_id, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TextureParameteri(texture_id, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);

            gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RGBA as i32, width as gl::types::GLsizei, height as gl::types::GLsizei, 0, gl::RGBA, gl::UNSIGNED_BYTE, ptr::null());
        }
    }

    ///
    /// Creates an empty MSAA texture
    ///
    pub fn create_empty_multisampled(&mut self, width: u16, height: u16, samples: usize) {
        unsafe {
            let texture_id = self.texture.texture_id;

            // Clamp the number of samples to the maximum supported by the driver
            let mut max_samples = 1;
            gl::GetIntegerv(gl::MAX_COLOR_TEXTURE_SAMPLES, &mut max_samples);
            let samples = max_samples.min(samples as i32);

            // Set up a MSAA texture
            gl::BindTexture(gl::TEXTURE_2D_MULTISAMPLE, texture_id);

            gl::TextureParameteri(texture_id, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TextureParameteri(texture_id, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);

            gl::TexImage2DMultisample(gl::TEXTURE_2D_MULTISAMPLE, samples, gl::RGBA, width as gl::types::GLsizei, height as gl::types::GLsizei, gl::FALSE);
        }
    }

    ///
    /// Associates an empty image with this texture
    ///
    pub fn create_monochrome(&mut self, width: u16, height: u16) {
        unsafe {
            let texture_id = self.texture.texture_id;

            gl::BindTexture(gl::TEXTURE_2D, texture_id);

            gl::TextureParameteri(texture_id, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TextureParameteri(texture_id, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);

            gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RED as i32, width as gl::types::GLsizei, height as gl::types::GLsizei, 0, gl::RED, gl::UNSIGNED_BYTE, ptr::null());
        }
    }

    ///
    /// Creates an empty MSAA texture
    ///
    pub fn create_monochrome_multisampled(&mut self, width: u16, height: u16, samples: usize) {
        unsafe {
            let texture_id = self.texture.texture_id;

            // Clamp the number of samples to the maximum supported by the driver
            let mut max_samples = 1;
            gl::GetIntegerv(gl::MAX_COLOR_TEXTURE_SAMPLES, &mut max_samples);
            let samples = max_samples.min(samples as i32);

            // Set up a MSAA texture
            gl::BindTexture(gl::TEXTURE_2D_MULTISAMPLE, texture_id);

            gl::TextureParameteri(texture_id, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TextureParameteri(texture_id, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);

            gl::TexImage2DMultisample(gl::TEXTURE_2D_MULTISAMPLE, samples, gl::RED, width as gl::types::GLsizei, height as gl::types::GLsizei, gl::FALSE);
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
