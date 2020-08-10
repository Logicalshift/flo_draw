use super::bindings::*;
use crate::buffer::*;

use metal;

use std::ffi::{c_void};

///
/// Manages a metal buffer
///
pub struct Buffer {
    /// The buffer that this is managing
    buffer: metal::Buffer
}

impl Buffer {
    ///
    /// Creates a buffer from some vertices
    ///
    pub fn from_vertices<VertexIterator: IntoIterator<Item=Vertex2D>>(device: &metal::Device, vertices: VertexIterator) -> Buffer {
        // Convert the vertices to Metal format
        let vertices = vertices.into_iter()
            .map(|vertex| MetalVertex2D::from(vertex))
            .collect::<Vec<_>>();

        // Generate the buffer
        let buffer = device.new_buffer_with_data(vertices.as_ptr() as *const c_void,
            (vertices.len() * std::mem::size_of::<MetalVertex2D>()) as u64,
            metal::MTLResourceOptions::CPUCacheModeDefaultCache | metal::MTLResourceOptions::StorageModeManaged);

        // Return the buffer object
        Buffer {
            buffer: buffer
        }
    }
}
