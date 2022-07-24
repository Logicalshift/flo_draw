use crate::buffer::*;

use wgpu;
use wgpu::util;
use wgpu::util::{DeviceExt};

use std::mem;
use std::slice;
use std::ffi::{c_void};

///
/// Converts a value to a WGPU buffer
///
pub (crate) trait ToWgpuBuffer {
    fn to_buffer(&self, device: &wgpu::Device, usage: wgpu::BufferUsages) -> wgpu::Buffer;
}

impl ToWgpuBuffer for Vec<Vertex2D> {
    #[inline]
    fn to_buffer(&self, device: &wgpu::Device, usage: wgpu::BufferUsages) -> wgpu::Buffer {
        let contents_void   = self.as_ptr() as *const c_void;
        let contents_len    = self.len() * mem::size_of::<Vertex2D>();
        let contents_u8     = unsafe { slice::from_raw_parts(contents_void as *const u8, contents_len) };

        device.create_buffer_init(&util::BufferInitDescriptor {
            label:      Some("Vec<Vertex2D>::to_buffer"),
            contents:   contents_u8,
            usage:      usage,
        })
    }
}

impl ToWgpuBuffer for Vec<u16> {
    #[inline]
    fn to_buffer(&self, device: &wgpu::Device, usage: wgpu::BufferUsages) -> wgpu::Buffer {
        let contents_void   = self.as_ptr() as *const u16;
        let contents_len    = self.len() * mem::size_of::<u16>();
        let contents_u8     = unsafe { slice::from_raw_parts(contents_void as *const u8, contents_len) };

        device.create_buffer_init(&util::BufferInitDescriptor {
            label:      Some("Vec<u16>::to_buffer"),
            contents:   contents_u8,
            usage:      usage,
        })
    }
}

impl ToWgpuBuffer for f32 {
    #[inline]
    fn to_buffer(&self, device: &wgpu::Device, usage: wgpu::BufferUsages) -> wgpu::Buffer {
        let contents_void   = self as *const f32;
        let contents_len    = mem::size_of::<f32>();
        let contents_u8     = unsafe { slice::from_raw_parts(contents_void as *const u8, contents_len) };

        device.create_buffer_init(&util::BufferInitDescriptor {
            label:      Some("f32::to_buffer"),
            contents:   contents_u8,
            usage:      usage,
        })
    }
}
