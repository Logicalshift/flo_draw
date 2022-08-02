use super::error::*;
use super::offscreen_trait::*;

use crate::action::*;
use crate::metal_renderer::*;

use metal;

use std::ffi::{c_void};

///
/// A metal offscreen render context
///
struct MetalOffscreenRenderContext {
    device: metal::Device
}

struct MetalOffscreenRenderTarget {
    device:         metal::Device,
    render_target:  RenderTarget,
    renderer:       MetalRenderer,
    width:          usize,
    height:         usize
}

///
/// Performs on-startup initialisation steps for offscreen rendering
///
/// Only required if not using a toolkit renderer (eg, in an HTTP renderer or command-line tool). Will likely replace
/// the bindings for any GUI toolkit, so this is not appropriate for desktop-type apps.
///
/// This version is the Metal version for Mac OS X
///
pub fn metal_initialize_offscreen_rendering() -> Result<impl OffscreenRenderContext, RenderInitError> {
    // Get the default metal device for the current system
    let device = metal::Device::system_default();
    let device = if let Some(device) = device { device } else { Err(RenderInitError::CannotOpenGraphicsDevice)? };

    // Result is a Metal offscreen render context
    Ok(MetalOffscreenRenderContext {
        device: device
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
pub fn initialize_offscreen_rendering() -> Result<impl OffscreenRenderContext, RenderInitError> {
    metal_initialize_offscreen_rendering()
}

impl OffscreenRenderContext for MetalOffscreenRenderContext {
    type RenderTarget = MetalOffscreenRenderTarget;

    ///
    /// Creates a new render target for this context
    ///
    fn create_render_target(&mut self, width: usize, height: usize) -> Self::RenderTarget {
        let device          = self.device.clone();
        let render_target   = RenderTarget::new(&self.device, width, height, RenderTargetType::StandardForReading);
        let renderer        = MetalRenderer::with_device(&self.device, true);

        MetalOffscreenRenderTarget {
            device:         device,
            render_target:  render_target,
            renderer:       renderer,
            width:          width,
            height:         height
        }
    }
}

impl OffscreenRenderTarget for MetalOffscreenRenderTarget {
    ///
    /// Sends render actions to this offscreen render target
    ///
    fn render<ActionIter: IntoIterator<Item=RenderAction>>(&mut self, actions: ActionIter) {
        let buffer          = self.renderer.render_to_buffer(actions, self.render_target.render_texture());

        let blit_encoder    = buffer.new_blit_command_encoder();
        blit_encoder.synchronize_resource(self.render_target.render_texture());
        blit_encoder.end_encoding();

        buffer.commit();
        buffer.wait_until_completed();
    }

    ///
    /// Consumes this render target and returns the realized pixels as a byte array
    ///
    fn realize(self) -> Vec<u8> {
        let mut result  = vec![0; self.width * self.height * 4];

        let texture     = self.render_target.render_texture();
        let region      = metal::MTLRegion {
            origin: metal::MTLOrigin    { x: 0, y: 0, z: 0 },
            size:   metal::MTLSize      { width: self.width as u64, height: self.height as u64, depth: 1 }
        };
        texture.get_bytes(result.as_mut_ptr() as *mut c_void, (self.width*4) as u64, region, 0);

        result
    }
}
