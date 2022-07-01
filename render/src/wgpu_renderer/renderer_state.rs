use crate::buffer::*;

use wgpu;
use wgpu::util;
use wgpu::util::{DeviceExt};

use std::mem;
use std::slice;
use std::sync::*;
use std::ffi::{c_void};

///
/// State for the WGPU renderer
///
pub (crate) struct RendererState {
    /// The command queue for the device
    queue:          Arc<wgpu::Queue>,

    /// The matrix transform buffer
    matrix_buffer:  wgpu::Buffer,

    /// The binding group for the matrix buffer
    matrix_binding: wgpu::BindGroup,
}

impl RendererState {
    ///
    /// Creates a default render state
    ///
    pub fn new(command_queue: Arc<wgpu::Queue>, device: &wgpu::Device) -> RendererState {
        // TODO: we can avoid re-creating some of these structures every frame: eg, the binding groups in particular

        // Create all the state structures
        let (matrix_buffer, matrix_binding) = Self::create_transform_buffer(&device);

        RendererState {
            queue:          command_queue,

            matrix_buffer:  matrix_buffer,
            matrix_binding: matrix_binding,
        }
    }

    ///
    /// Updates the contents of the matrix buffer for this renderer
    ///
    #[inline]
    pub fn write_matrix(&self, new_matrix: &Matrix) {
        let matrix_void     = new_matrix.0.as_ptr() as *const c_void;
        let matrix_len      = mem::size_of::<[[f32; 4]; 4]>();
        let matrix_u8       = unsafe { slice::from_raw_parts(matrix_void as *const u8, matrix_len) };

        self.queue.write_buffer(&self.matrix_buffer, 0, matrix_u8);
    }

    ///
    /// Sets up the transform buffer and layout
    ///
    fn create_transform_buffer(device: &wgpu::Device) -> (wgpu::Buffer, wgpu::BindGroup) {
        // Convert the matrix to a u8 pointer
        let matrix          = Matrix::identity();
        let matrix_void     = matrix.0.as_ptr() as *const c_void;
        let matrix_len      = mem::size_of::<[[f32; 4]; 4]>();
        let matrix_u8       = unsafe { slice::from_raw_parts(matrix_void as *const u8, matrix_len) };

        // Load into a buffer
        let matrix_buffer   = device.create_buffer_init(&util::BufferInitDescriptor {
            label:      Some("matrix_buffer"),
            contents:   matrix_u8,
            usage:      wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create a binding group
        let matrix_layout   = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label:      Some("matrix_layout"),
            entries:    &[
                wgpu::BindGroupLayoutEntry {
                    binding:    0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    count:      None,
                    ty:         wgpu::BindingType::Buffer {
                        ty:                 wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size:   None,
                    }    
                }
            ],
        });

        let matrix_binding  = device.create_bind_group(&wgpu::BindGroupDescriptor {
             label:     Some("matrix_binding"),
             layout:    &matrix_layout,
             entries:   &[
                wgpu::BindGroupEntry {
                    binding:    0,
                    resource:   matrix_buffer.as_entire_binding(),
                }
             ]
        });

        (matrix_buffer, matrix_binding)
    }
}