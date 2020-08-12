use super::bindings::*;
use crate::buffer::*;

use metal;
use cocoa::foundation::{NSRange};

use std::ptr;
use std::mem;
use std::ops::{Deref};
use std::ffi::{c_void};

///
/// Manages a metal buffer containing a matrix
///
pub struct MatrixBuffer {
    /// The buffer that this is managing
    buffer: metal::Buffer
}

impl MatrixBuffer {
    ///
    /// Creates a matrix buffer from a matrix
    ///
    pub fn from_matrix(device: &metal::Device, matrix: Matrix) -> MatrixBuffer {
        // Convert to the internal representation
        let matrix = matrix_float4x4::from(matrix);

        unsafe {
            // Generate a metal buffer containing the matrix
            let matrix_ptr: *const matrix_float4x4  = &matrix;
            let buffer                              = device.new_buffer_with_data(matrix_ptr as *const c_void,
                std::mem::size_of::<matrix_float4x4>() as u64,
                metal::MTLResourceOptions::CPUCacheModeDefaultCache | metal::MTLResourceOptions::StorageModeManaged);

            // Return the buffer object
            MatrixBuffer {
                buffer: buffer
            }
        }
    }

    ///
    /// Updates the matrix in this buffer
    ///
    pub fn set_matrix(&mut self, matrix: Matrix) {
        // Convert to the internal representation
        let matrix = matrix_float4x4::from(matrix);

        // Copy the matrix to the buffer
        unsafe {
            let content = self.buffer.contents();
            ptr::copy(&matrix, content as *mut matrix_float4x4, mem::size_of::<matrix_float4x4>());
        }

        // Tell the buffer its been modified
        self.buffer.did_modify_range(NSRange::new(0, mem::size_of::<matrix_float4x4>() as u64));
    }
}

impl Deref for MatrixBuffer {
    type Target = metal::Buffer;

    fn deref(&self) -> &metal::Buffer {
        &self.buffer
    }
}
