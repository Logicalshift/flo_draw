use gl;

use std::ops::{Deref};

///
/// Abstraction for an OpenGL vertex array object
///
pub struct VertexArray {
    vertex_array_object: gl::types::GLuint
}

impl VertexArray {
    ///
    /// Creates a new vertex array
    ///
    pub fn new() -> VertexArray {
        unsafe {
            // Create the array
            let mut new_array: gl::types::GLuint = 0;
            gl::GenVertexArrays(1, &mut new_array);

            // Encapsulate into the structure
            VertexArray {
                vertex_array_object: new_array
            }
        }
    }
}

impl Deref for VertexArray {
    type Target = gl::types::GLuint;

    fn deref(&self) -> &gl::types::GLuint {
        &self.vertex_array_object
    }
}

impl Drop for VertexArray {
    fn drop(&mut self) { 
        unsafe {
            gl::DeleteVertexArrays(1, &mut self.vertex_array_object);
        }
    }
}
