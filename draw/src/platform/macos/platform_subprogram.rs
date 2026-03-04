use crate::platform::*;

use flo_canvas_events::*;
use flo_scene::*;
use flo_stream::*;

use futures::prelude::*;

use std::sync::*;

///
/// Attaches a WGPU FloDrawView (handles extra events and adds some extra rendering rules)
///
#[cfg(feature="render-wgpu")]
async fn attach_wgpu(window_id: ::winit::window::WindowId, scale: f64, _events: Arc<WeakPublisher<DrawEvent>>, platform_window: Arc<dyn PlatformWindow>) {
    use flo_render::{WgpuRenderer};
    use crate::wgpu::*;

    // Dispatch the 'create' event to the winit thread (the wgpu adapter, etc, can't leave the thread)
    winit_thread().send_event(WinitThreadEvent::RunProcess(Box::new(move || async move {
        // Create a rendering view for OS X
        let draw_view = FloDrawView::new();

        // Initialise as the drawing surface
        let backend         = wgpu::Backends::from_env().unwrap_or_else(|| wgpu::Backends::PRIMARY);
        let instance        = wgpu::Instance::new(&wgpu::InstanceDescriptor { backends: backend, ..Default::default() });
        let surface         = draw_view.create_surface(&instance);
        let adapter         = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference:       wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            compatible_surface:     Some(&surface),
        }).await.expect("Could not acquire an adapter for winit/wgpu");

        // Fetch the device and the queue
        let features        = wgpu::Features::empty();
        #[cfg(feature="wgpu-profiler")] let features = features | GpuProfiler::ALL_WGPU_TIMER_FEATURES;
        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
            label:              None,
            required_features:  features,
            required_limits:    wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits()),
            ..Default::default()
        }).await.expect("Create WGPU device and queue");

        // Create the WGPU renderer
        let device          = Arc::new(device);
        let queue           = Arc::new(queue);
        let surface         = Arc::new(surface);
        let adapter         = Arc::new(adapter);
        let renderer        = WgpuRenderer::from_surface(Arc::clone(&device), Arc::clone(&queue), Arc::clone(&surface), Arc::clone(&adapter));

        // TODO: need to send the wgpu details back to the winit thread
        //window_lock.device      = Some(device);
        //window_lock.instance    = Some(instance);
        //window_lock.renderer    = Some(renderer);

        // Attach to the window (need to release the lock while we do this)
        //mem::drop(window_lock);
        draw_view.attach_to(&Arc::new(Mutex::new(platform_window)));
        //window_lock = window.lock().unwrap();
        //window_lock.draw_view   = Some(draw_view);
    }.boxed_local()), "Start FloDrawView".into()));
}

///
/// Subprogram that manages windows on OS X
///
pub async fn macos_platform_subprogram(input: InputStream<WinitEvents>, context: SceneContext, winit_events: Subscriber<WinitEvents>) {
    // Read from the subscription or from the input (we need to read from the input to ensure that the scene becomes idle)
    let mut winit_events = stream::select(winit_events, input);

    // Monitor the winit events for 
    while let Some(event) = winit_events.next().await {
        match event {
            WinitEvents::CreatedWindow { window_id, scale, events, platform_window } => {
                #[cfg(feature="render-wgpu")]
                attach_wgpu(window_id, scale, events, platform_window).await;
            }
        }
    }
}
