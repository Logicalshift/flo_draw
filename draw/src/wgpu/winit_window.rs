use crate::events::*;
use crate::window_properties::*;

use flo_stream::*;
use flo_render::*;
use flo_binding::*;

use wgpu;
use winit::dpi::{LogicalSize};
use winit::window::{Window, Fullscreen};
use futures::prelude::*;
use futures::task::{Poll, Context};

use std::pin::*;
use std::sync::*;

use std::fmt;

#[cfg(feature="profile")]
use std::time::{Duration, Instant};

#[cfg(feature="wgpu-profiler")]
use wgpu_profiler::{GpuProfiler};

#[cfg(target_os="macos")]
use super::platform_macos::*;

///
/// Manages the state of a Winit window
///
pub struct WinitWindow {
    /// The window that this is acting for
    window: Option<Arc<Window>>,

    /// The device that this is acting for
    device: Option<Arc<wgpu::Device>>,

    /// The WGPU instance used by this window
    instance: Option<wgpu::Instance>,

    /// The renderer for this window (or none if there isn't one yet)
    renderer: Option<WgpuRenderer<'static>>,

    /// The drawing view, if it has been created
    #[cfg(target_os="macos")]
    draw_view: Option<Retained<FloDrawView>>,
}

impl WinitWindow {
    ///
    /// Creates a new winit window
    ///
    pub fn new(window: Arc<Window>) -> WinitWindow {
        WinitWindow {
            window:     Some(window),
            device:     None,
            instance:   None,
            renderer:   None,
            #[cfg(target_os="macos")]
            draw_view:  None,
        }
    }

    ///
    /// Returns the underlying winit window object
    ///
    pub (super) fn window(&self) -> Option<Arc<Window>> {
        self.window.clone()
    }
}

///
/// Sends render actions to a window
///
/// Render actions are sent via the stream in `render_actions`. When a new frame is presented, a `DrawEvent::NewFrame` is sent to the event publisher. Finally the window 
/// properties are watched and used to update the window properties.
///
pub (super) async fn send_actions_to_window<RenderStream, EventPublisher>(window: WinitWindow, render_actions: RenderStream, events: EventPublisher, window_properties: WindowProperties)
where
    RenderStream:   Unpin + Stream<Item=Vec<RenderAction>>,
    EventPublisher: MessagePublisher<Message=DrawEvent>,
{
    // Read events from the render actions list
    let window              = Arc::new(Mutex::new(window));
    let mut events          = events;
    let window_actions      = WindowUpdateStream { 
        render_stream:      render_actions, 
        title_stream:       follow(window_properties.title),
        size:               follow(window_properties.size),
        fullscreen:         follow(window_properties.fullscreen),
        has_decorations:    follow(window_properties.has_decorations),
        mouse_pointer:      follow(window_properties.mouse_pointer)
    };
    let mut window_actions  = window_actions.ready_chunks(100);

    while let Some(next_action_set) = window_actions.next().await {
        let mut send_new_frame = false;

        for next_action in next_action_set {
            match next_action {
                WindowUpdate::Render(next_action)   => {
                    let mut window_lock = window.lock().unwrap();

                    // Do nothing if there are no actions
                    if next_action.len() == 0 {
                        events.publish(DrawEvent::NewFrame).await;
                        continue;
                    }

                    // Create the subview
                    #[cfg(target_os="macos")]
                    if let (Some(_winit_window), None) = (&window_lock.window, &window_lock.draw_view) {
                        use std::mem;

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
                        }, None).await.expect("Create WGPU device and queue");

                        // Create the WGPU renderer
                        let device          = Arc::new(device);
                        let queue           = Arc::new(queue);
                        let surface         = Arc::new(surface);
                        let adapter         = Arc::new(adapter);
                        let renderer        = WgpuRenderer::from_surface(Arc::clone(&device), Arc::clone(&queue), Arc::clone(&surface), Arc::clone(&adapter));

                        window_lock.device      = Some(device);
                        window_lock.instance    = Some(instance);
                        window_lock.renderer    = Some(renderer);

                        // Attach to the window (need to release the lock while we do this)
                        mem::drop(window_lock);
                        draw_view.attach_to(&window);
                        window_lock = window.lock().unwrap();
                        window_lock.draw_view   = Some(draw_view);

                        // First frame has been displayed
                        send_new_frame = true;
                    }

                    // Referencing the value in the lock makes borrowing the contents easier
                    let window_lock = &mut *window_lock;

                    // Create the renderer if it doesn't already exist
                    if let (Some(winit_window), None) = (&window_lock.window, &window_lock.renderer) {
                        // Create a new WGPU instance, surface and adapter
                        let winit_window    = winit_window.clone();

                        let backend         = wgpu::Backends::from_env().unwrap_or_else(|| wgpu::Backends::PRIMARY);
                        let instance        = wgpu::Instance::new(&wgpu::InstanceDescriptor { backends: backend, ..Default::default() });
                        let surface         = instance.create_surface(winit_window).expect("wgpu surface");
                        let adapter         = instance.request_adapter(&wgpu::RequestAdapterOptions {
                            power_preference:       wgpu::PowerPreference::default(),
                            force_fallback_adapter: false,
                            compatible_surface:     Some(&surface),
                        }).await.expect("Could not acquire an adapter for winit/wgpu");

                        // Fetch the device and the queue
                        let features = wgpu::Features::empty();
                        #[cfg(feature="wgpu-profiler")] let features = features | GpuProfiler::ALL_WGPU_TIMER_FEATURES;
                        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
                            label:              None,
                            required_features:  features,
                            required_limits:    wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits()),
                            ..Default::default()
                        }, None).await.expect("Create WGPU device and queue");

                        // Create the WGPU renderer
                        let device              = Arc::new(device);
                        let queue               = Arc::new(queue);
                        let surface             = Arc::new(surface);
                        let adapter             = Arc::new(adapter);
                        let renderer            = WgpuRenderer::from_surface(Arc::clone(&device), Arc::clone(&queue), Arc::clone(&surface), Arc::clone(&adapter));

                        window_lock.device      = Some(device);
                        window_lock.instance    = Some(instance);
                        window_lock.renderer    = Some(renderer);

                        // First frame has been displayed
                        send_new_frame = true;
                    }

                    if let (Some(winit_window), Some(renderer)) = (&window_lock.window, &mut window_lock.renderer) {
                        // Set up to render at the current size
                        let size    = winit_window.inner_size();
                        let width   = size.width;
                        let height  = size.height;

                        #[cfg(target_os="macos")]
                        {
                            CATransaction::begin();
                            CATransaction::setAnimationDuration(0.0);
                            CATransaction::setDisableActions(true);
                        }

                        renderer.prepare_to_render(width, height);      // TODO: if we switch between presentation modes we could use immediate while resizing and AutoVsync in 'normal' circumstances

                        // Send the commands to the renderer
                        let maybe_next_frame = renderer.render_to_surface(next_action);

                        // Notify that a new frame has been drawn if show_frame_buffer is set
                        if let Some(next_frame) = maybe_next_frame {
                            #[cfg(feature="profile")]
                            let start_time = Instant::now();

                            next_frame.present();

                            #[cfg(feature="profile")]
                            println!("WINIT: time to present frame {}µs", Instant::now().duration_since(start_time).as_micros());

                            // Trigger the 'NewFrame' event when done
                            send_new_frame = true;
                        }

                        #[cfg(target_os="macos")]
                        CATransaction::commit();
                    }
                }

                WindowUpdate::SetTitle(new_title)   => {
                    let window_lock = window.lock().unwrap();
                    if let Some(winit_window) = &window_lock.window {
                        winit_window.set_title(&new_title);
                    }
                }

                WindowUpdate::SetSize((size_x, size_y)) => {
                    let window_lock = window.lock().unwrap();
                    if let Some(winit_window) = &window_lock.window {
                        let _ = winit_window.request_inner_size(LogicalSize::new(size_x as f64, size_y as _));
                    }
                }

                WindowUpdate::SetFullscreen(is_fullscreen) => {
                    let window_lock = window.lock().unwrap();
                    let fullscreen = if is_fullscreen { Some(Fullscreen::Borderless(None)) } else { None };
                    if let Some(winit_window) = &window_lock.window {
                        winit_window.set_fullscreen(fullscreen);
                    }
                }

                WindowUpdate::SetHasDecorations(decorations) => {
                    let window_lock = window.lock().unwrap();
                    if let Some(winit_window) = &window_lock.window {
                        winit_window.set_decorations(decorations);
                    }
                }

                WindowUpdate::SetMousePointer(MousePointer::None) => {
                    let window_lock = window.lock().unwrap();
                    if let Some(winit_window) = &window_lock.window {
                        winit_window.set_cursor_visible(false);
                    }
                }

                WindowUpdate::SetMousePointer(MousePointer::SystemDefault) => {
                    let window_lock = window.lock().unwrap();
                    if let Some(winit_window) = &window_lock.window {
                        winit_window.set_cursor_visible(true);
                    }
                }
            }
        }

        // If any rendering occurred in the last batch of events, send the new frame result
        if send_new_frame {
            // We only send one of these per batch, in case multiple frames are displayed during one batch of events for any reason (this reduces the
            // amount that the rendering can get behind)
            events.publish(DrawEvent::NewFrame).await;
        }
    }

    // Window will close once the render actions are finished as we drop it here
}

///
/// The list of update events that can occur to a window
///
enum WindowUpdate {
    Render(Vec<RenderAction>),
    SetTitle(String),
    SetSize((u64, u64)),
    SetFullscreen(bool),
    SetHasDecorations(bool),
    SetMousePointer(MousePointer)
}

impl fmt::Debug for WindowUpdate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::WindowUpdate::*;

        match self {
            Render(actions)             => write!(f, "Render({} actions)", actions.len()),
            SetTitle(title)             => write!(f, "SetTitle({})", title),
            SetSize(sz)                 => write!(f, "SetSize({:?})", sz),
            SetFullscreen(val)          => write!(f, "SetFullscreen({:?})", val),
            SetHasDecorations(val)      => write!(f, "SetHasDecorations({:?})", val),
            SetMousePointer(ptr)        => write!(f, "SetMousePointer({:?})", ptr),
        }
    }
}

///
/// Stream that merges the streams from the window properties and the renderer into a single stream
///
struct WindowUpdateStream<TRenderStream, TTitleStream, TSizeStream, TFullscreenStream, TDecorationStream, TMousePointerStream> {
    render_stream:      TRenderStream,
    title_stream:       TTitleStream,
    size:               TSizeStream,
    fullscreen:         TFullscreenStream,
    has_decorations:    TDecorationStream,
    mouse_pointer:      TMousePointerStream
}

impl<TRenderStream, TTitleStream, TSizeStream, TFullscreenStream, TDecorationStream, TMousePointerStream> Stream for WindowUpdateStream<TRenderStream, TTitleStream, TSizeStream, TFullscreenStream, TDecorationStream, TMousePointerStream>
where
    TRenderStream:          Unpin + Stream<Item=Vec<RenderAction>>,
    TTitleStream:           Unpin + Stream<Item=String>,
    TSizeStream:            Unpin + Stream<Item=(u64, u64)>,
    TFullscreenStream:      Unpin + Stream<Item=bool>,
    TDecorationStream:      Unpin + Stream<Item=bool>,
    TMousePointerStream:    Unpin + Stream<Item=MousePointer> 
{
    type Item = WindowUpdate;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Poll each stream in turn to see if they have an item

        // Rendering instructions have priority
        match self.render_stream.poll_next_unpin(context) {
            Poll::Ready(Some(item)) => { return Poll::Ready(Some(WindowUpdate::Render(item))); }
            Poll::Ready(None)       => { return Poll::Ready(None); }
            Poll::Pending           => { }
        }

        // The various binding streams
        match self.title_stream.poll_next_unpin(context) {
            Poll::Ready(Some(item)) => { return Poll::Ready(Some(WindowUpdate::SetTitle(item))); }
            Poll::Ready(None)       => { return Poll::Ready(None); }
            Poll::Pending           => { }
        }

        match self.size.poll_next_unpin(context) {
            Poll::Ready(Some(item)) => { return Poll::Ready(Some(WindowUpdate::SetSize(item))); }
            Poll::Ready(None)       => { return Poll::Ready(None); }
            Poll::Pending           => { }
        }

        match self.fullscreen.poll_next_unpin(context) {
            Poll::Ready(Some(item)) => { return Poll::Ready(Some(WindowUpdate::SetFullscreen(item))); }
            Poll::Ready(None)       => { return Poll::Ready(None); }
            Poll::Pending           => { }
        }

        match self.has_decorations.poll_next_unpin(context) {
            Poll::Ready(Some(item)) => { return Poll::Ready(Some(WindowUpdate::SetHasDecorations(item))); }
            Poll::Ready(None)       => { return Poll::Ready(None); }
            Poll::Pending           => { }
        }

        match self.mouse_pointer.poll_next_unpin(context) {
            Poll::Ready(Some(item)) => { return Poll::Ready(Some(WindowUpdate::SetMousePointer(item))); }
            Poll::Ready(None)       => { return Poll::Ready(None); }
            Poll::Pending           => { }
        }

        // No stream matched anything
        Poll::Pending
    }
}
