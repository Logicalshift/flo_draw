use winit;

use std::sync::*;

#[cfg(feature = "render-wgpu")] use flo_render::{WgpuRenderer};

///
/// Trait used to retrieve values from a window for the OS X platform
///
pub trait PlatformWindow : Send + Sync {
    ///
    /// Retrieves the winit window for this platform
    ///
    fn window(&self) -> Option<Arc<winit::window::Window>>;

    ///
    /// Sets the WGPU rendering surface (if this is a WGPU window), overriding the default behaviour
    ///
    #[cfg(feature = "render-wgpu")]
    fn set_wgpu_surface(&self, _device: Arc<wgpu::Device>, _instance: wgpu::Instance, _renderer: WgpuRenderer<'static>) { }
}

impl PlatformWindow for Arc<dyn PlatformWindow> {
    #[inline]
    fn window(&self) -> Option<Arc<winit::window::Window>> {
        (**self).window()
    }

    #[cfg(feature = "render-wgpu")]
    fn set_wgpu_surface(&self, device: Arc<wgpu::Device>, instance: wgpu::Instance, renderer: WgpuRenderer<'static>) {
        (**self).set_wgpu_surface(device, instance, renderer);
    }
}

impl PlatformWindow for Box<dyn PlatformWindow> {
    #[inline]
    fn window(&self) -> Option<Arc<winit::window::Window>> {
        (**self).window()
    }

    #[cfg(feature = "render-wgpu")]
    fn set_wgpu_surface(&self, device: Arc<wgpu::Device>, instance: wgpu::Instance, renderer: WgpuRenderer<'static>) {
        (**self).set_wgpu_surface(device, instance, renderer);
    }
}
