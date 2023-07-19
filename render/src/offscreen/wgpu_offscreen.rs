use super::error::*;
use super::offscreen_trait::*;

use crate::action::*;
use crate::wgpu_renderer::*;

use ::desync::*;
use futures::prelude::*;
use once_cell::sync::{Lazy};

use wgpu;

use std::num::*;
use std::sync::*;

static WGPU_BACKGROUND: Lazy<Desync<()>> = Lazy::new(|| Desync::new(()));

///
/// A WGPU offscreen render context
///
struct WgpuOffscreenRenderContext {
    device:     Arc<wgpu::Device>,
    adapter:    Arc<wgpu::Adapter>,
    queue:      Arc<wgpu::Queue>,
}

struct WgpuOffscreenRenderTarget {
    texture:    Arc<wgpu::Texture>,
    device:     Arc<wgpu::Device>,
    queue:      Arc<wgpu::Queue>,
    renderer:   WgpuRenderer,
    size:       (u32, u32),
}

///
/// Performs on-startup initialisation steps for offscreen rendering using the WGPU implementation
///
/// Only required if not using a toolkit renderer (eg, in an HTTP renderer or command-line tool). Will likely replace
/// the bindings for any GUI toolkit, so this is not appropriate for desktop-type apps.
///
/// This version is the Metal version for Mac OS X
///
pub async fn wgpu_initialize_offscreen_rendering() -> Result<impl OffscreenRenderContext, RenderInitError> {
    // Create a new WGPU instance and adapter
    let instance    = wgpu::Instance::new(wgpu::InstanceDescriptor { backends: wgpu::Backends::all(), dx12_shader_compiler: wgpu::Dx12Compiler::default() });
    let adapter     = instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference:       wgpu::PowerPreference::default(),
        force_fallback_adapter: false,
        compatible_surface:     None,
    }).await.unwrap();

    // Fetch the device and the queue
    let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
            label:      None,
            features:   wgpu::Features::empty(),
            limits:     wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits())
        }, None).await.unwrap();

    // Result is a WGPU offscreen render context
    Ok(WgpuOffscreenRenderContext {
        device:     Arc::new(device),
        adapter:    Arc::new(adapter),
        queue:      Arc::new(queue),
    })
}

///
/// Performs on-startup initialisation steps for offscreen rendering
///
/// Only required if not using a toolkit renderer (eg, in an HTTP renderer or command-line tool). Will likely replace
/// the bindings for any GUI toolkit, so this is not appropriate for desktop-type apps.
///
/// This version is the Metal version for Mac OS X
///
#[cfg(not(any(feature="opengl", feature="osx-metal")))]
pub fn initialize_offscreen_rendering() -> Result<impl OffscreenRenderContext, RenderInitError> {
    WGPU_BACKGROUND.future_desync(|_| async { wgpu_initialize_offscreen_rendering().await }.boxed()).sync().unwrap()
}

impl OffscreenRenderContext for WgpuOffscreenRenderContext {
    type RenderTarget = WgpuOffscreenRenderTarget;

    ///
    /// Creates a new render target for this context
    ///
    fn create_render_target(&mut self, width: usize, height: usize) -> Self::RenderTarget {
        // Create a texture to render on
        let target_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label:              Some("WgpuOffscreenRenderTarget"),
            size:               wgpu::Extent3d { width: width as _, height: height as _, depth_or_array_layers: 1 },
            mip_level_count:    1,
            sample_count:       1,
            dimension:          wgpu::TextureDimension::D2,
            format:             wgpu::TextureFormat::Rgba8Unorm,
            usage:              wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats:       &[wgpu::TextureFormat::Rgba8Unorm],
        });

        let target_texture = Arc::new(target_texture);

        // Create a renderer that will write to this texture
        let renderer = WgpuRenderer::from_texture(Arc::clone(&self.device), Arc::clone(&self.queue), Arc::clone(&target_texture), Arc::clone(&self.adapter), wgpu::TextureFormat::Rgba8Unorm, (width as _, height as _));

        // Build the render target
        WgpuOffscreenRenderTarget {
            device:     Arc::clone(&self.device),
            queue:      Arc::clone(&self.queue),
            size:       (width as _, height as _),
            texture:    target_texture,
            renderer:   renderer,
        }
    }
}

impl OffscreenRenderTarget for WgpuOffscreenRenderTarget {
    ///
    /// Sends render actions to this offscreen render target
    ///
    #[inline]
    fn render<ActionIter: IntoIterator<Item=RenderAction>>(&mut self, actions: ActionIter) {
        self.renderer.render_to_surface(actions);
    }

    ///
    /// Consumes this render target and returns the realized pixels as a byte array
    ///
    fn realize(self) -> Vec<u8> {
        // Create a buffer to store the result
        let bytes_per_row   = (((self.size.0 * 4 - 1) / 256) + 1) * 256;
        let buffer          = self.device.create_buffer(&wgpu::BufferDescriptor {
            label:              Some("WgpuOffscreenRenderTarget::realize"),
            size:               (bytes_per_row as u64) * (self.size.1 as u64),
            usage:              wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // Copy the texture to the buffer
        let mut encoder     = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("WgpuOffscreenRenderTarget::realize") });
        let buffer_copy     = wgpu::ImageCopyBuffer { buffer: &buffer, layout: wgpu::ImageDataLayout { offset: 0, bytes_per_row: Some(bytes_per_row), rows_per_image: None } };
        encoder.copy_texture_to_buffer(self.texture.as_image_copy(), buffer_copy, wgpu::Extent3d { width: self.size.0, height: self.size.1, depth_or_array_layers: 1 });
        self.queue.submit(Some(encoder.finish()));

        // Take the whole buffer as a slice
        let buffer_slice    = buffer.slice(..);

        // Create the final target
        let ready           = Arc::new(Mutex::new(false));

        // Map the buffer to memory, with a callback that writes the result
        let ready_clone     = Arc::clone(&ready);
        buffer_slice.map_async(wgpu::MapMode::Read, move |_err| { *ready_clone.lock().unwrap() = true; });

        // Poll until the buffer is ready
        while *ready.lock().unwrap() == false {
            self.device.poll(wgpu::Maintain::Wait);
        }

        // Prepare to write the buffer
        let mut result      = vec![0; (self.size.0 * self.size.1 * 4) as usize];

        // Poll for the result
        let mapped_buffer   = buffer_slice.get_mapped_range();

        // Copy to a Vec<u8>
        let row_len = (self.size.0 * 4) as usize;
        for row in 0..self.size.1 {
            let buffer_row_start    = (row * bytes_per_row) as usize;
            let row_start           = ((self.size.1 - 1 - row) * self.size.0 * 4) as usize;

            result[row_start..(row_start+row_len)].copy_from_slice(&mapped_buffer[buffer_row_start..(buffer_row_start+row_len)]);
        }

        result
    }
}
