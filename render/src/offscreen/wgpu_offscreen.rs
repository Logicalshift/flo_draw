use super::error::*;
use super::offscreen_trait::*;

use crate::action::*;
use crate::wgpu_renderer::*;

use ::desync::*;

use wgpu;
use futures::prelude::*;

use std::sync::*;

lazy_static! {
    static ref WGPU_BACKGROUND: Desync<()> = Desync::new(());
}

///
/// A WGPU offscreen render context
///
struct WgpuOffscreenRenderContext {
    instance:   Arc<wgpu::Instance>,
    device:     Arc<wgpu::Device>,
    adapter:    Arc<wgpu::Adapter>,
    queue:      Arc<wgpu::Queue>,
}

struct WgpuOffscreenRenderTarget {

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
    let instance    = wgpu::Instance::new(wgpu::Backends::all());
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
        instance:   Arc::new(instance),
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
pub fn initialize_offscreen_rendering() -> Result<impl OffscreenRenderContext, RenderInitError> {
    WGPU_BACKGROUND.future_desync(|_| async { wgpu_initialize_offscreen_rendering().await }.boxed()).sync().unwrap()
}

impl OffscreenRenderContext for WgpuOffscreenRenderContext {
    type RenderTarget = WgpuOffscreenRenderTarget;

    ///
    /// Creates a new render target for this context
    ///
    fn create_render_target(&mut self, width: usize, height: usize) -> Self::RenderTarget {
        unimplemented!("create_render_target")
    }
}

impl OffscreenRenderTarget for WgpuOffscreenRenderTarget {
    ///
    /// Sends render actions to this offscreen render target
    ///
    fn render<ActionIter: IntoIterator<Item=RenderAction>>(&mut self, actions: ActionIter) {
        unimplemented!("render")
    }

    ///
    /// Consumes this render target and returns the realized pixels as a byte array
    ///
    fn realize(self) -> Vec<u8> {
        unimplemented!("realize")
    }
}
