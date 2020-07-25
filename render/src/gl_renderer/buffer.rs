use gl;

use std::mem;
use std::ops::{Deref};
use std::ffi::{c_void};

///
/// Abstraction of an OpenGL buffer object
///
pub struct Buffer {
    buffer: gl::types::GLuint
}

impl Buffer {
    ///
    /// Creates a new buffer
    ///
    pub fn new() -> Buffer {
        unsafe {
            let mut new_buffer = 0;
            gl::GenBuffers(1, &mut new_buffer);

            Buffer {
                buffer: new_buffer
            }
        }
    }

    ///
    /// Fills the buffer with static draw data
    ///
    pub fn static_draw<TData>(&mut self, data: &[TData])
    where TData: Sized {
        unsafe {
            gl::BindBuffer(gl::ARRAY_BUFFER, self.buffer);
            gl::BufferData(
                gl::ARRAY_BUFFER, 
                (mem::size_of::<TData>() * data.len()) as isize, 
                data.as_ptr() as *const c_void, 
                gl::STATIC_DRAW);
        }
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &mut self.buffer);
        }
    }
}

impl Deref for Buffer {
    type Target = gl::types::GLuint;

    fn deref(&self) -> &gl::types::GLuint {
        &self.buffer
    }
}
