use crate::events::*;
use crate::window_properties::*;

use flo_canvas::*;
use flo_stream::*;
use flo_render_software::draw::*;
use flo_render_software::pixel::*;
use flo_render_software::render::*;
use flo_render_software::scanplan::*;
use flo_binding::*;

use softbuffer;
use winit::dpi::{LogicalSize};
use winit::window::{Window, Fullscreen};
use futures::prelude::*;
use futures::task::{Poll, Context};
use ::desync::*;

use std::pin::*;
use std::sync::*;

use std::num::{NonZeroU32};
use std::fmt;

#[cfg(feature="profile")]
use std::time::{Duration, Instant};

///
/// Manages the state of a Winit window
///
pub struct WinitWindow {
    /// The window that this is acting for
    window: Option<Arc<Window>>,

    /// THe softbuffer context for the window (or None if it's not set up yet)
    context: Option<softbuffer::Context<Arc<Window>>>,

    /// The softbuffer surface that we're rendering on
    surface: Option<softbuffer::Surface<Arc<Window>, Arc<Window>>>,

    /// The bounds of the canvas to render in the window
    viewport_bounds: ViewportBounds,
}

impl WinitWindow {
    ///
    /// Creates a new winit window
    ///
    pub fn new(window: Arc<Window>) -> WinitWindow {
        WinitWindow {
            window:          Some(window),
            context:         None,
            surface:         None,
            viewport_bounds: ViewportBounds::All,
        }
    }
}

///
/// Sends drawing actions to a window
///
pub (super) async fn send_drawing_actions_to_window<DrawStream, EventPublisher>(window: WinitWindow, drawing_actions: DrawStream, events: EventPublisher, window_properties: WindowProperties)
where
    DrawStream:     Unpin + Stream<Item=Arc<Vec<Draw>>>,
    EventPublisher: MessagePublisher<Message=DrawEvent>,
{
    // Read events from the render actions list
    let mut window          = window;
    let mut events          = events;
    let window_actions      = WindowUpdateStream { 
        draw_stream:        drawing_actions, 
        title_stream:       follow(window_properties.title),
        size:               follow(window_properties.requested_size),
        fullscreen:         follow(window_properties.fullscreen),
        has_decorations:    follow(window_properties.has_decorations),
        mouse_pointer:      follow(window_properties.mouse_pointer),
        viewport_bounds:    follow(window_properties.viewport_bounds),
    };
    let mut window_actions      = window_actions.ready_chunks(100);
    let canvas_drawing          = CanvasDrawing::<F32LinearPixel, 4>::empty();
    let canvas_drawing          = Desync::new(canvas_drawing);
    let mut active_transform    = Transform2D::identity();

    while let Some(next_action_set) = window_actions.next().await {
        let mut send_new_frame          = false;
        let mut update_canvas_transform = false;
        let mut redraw_canvas           = false;

        for next_action in next_action_set {
            match next_action {
                WindowUpdate::Draw(next_action) => {
                    // Create the renderer if it doesn't already exist
                    if let (Some(winit_window), None) = (&window.window, &window.context) {
                        // Create a new softbuffer context
                        let winit_window        = winit_window.clone();
                        let softbuffer_context  = softbuffer::Context::new(winit_window.clone()).unwrap();
                        let softbuffer_surface  = softbuffer::Surface::new(&softbuffer_context, winit_window.clone()).unwrap();

                        window.context = Some(softbuffer_context);
                        window.surface = Some(softbuffer_surface);
                    }

                    // Queue up a render later on
                    redraw_canvas = true;

                    // Process the drawing instructions in the canvas (without doing the render step)
                    let new_transform;
                    (window, new_transform) = canvas_drawing.future_desync(move |canvas_drawing| async move {
                        canvas_drawing.draw(Arc::unwrap_or_clone(next_action).into_iter());
                        (window, canvas_drawing.active_transform())
                    }.boxed()).await.unwrap();

                    // Trigger the 'NewFrame' event when we're done processing the events
                    send_new_frame = true;

                    if active_transform != new_transform {
                        active_transform        = new_transform;
                        update_canvas_transform = true;
                    }
                }
                
                WindowUpdate::SetViewportBounds(new_bounds) => {
                    window.viewport_bounds = new_bounds;
                }

                WindowUpdate::SetTitle(new_title)   => {
                    if let Some(winit_window) = &window.window {
                        winit_window.set_title(&new_title);
                    }
                }

                WindowUpdate::SetSize((size_x, size_y)) => {
                    if let Some(winit_window) = &window.window {
                        let _ = winit_window.request_inner_size(LogicalSize::new(size_x as f64, size_y as _));
                    }
                }

                WindowUpdate::SetFullscreen(is_fullscreen) => {
                    let fullscreen = if is_fullscreen { Some(Fullscreen::Borderless(None)) } else { None };
                    if let Some(winit_window) = &window.window {
                        winit_window.set_fullscreen(fullscreen);
                    }
                }

                WindowUpdate::SetHasDecorations(decorations) => {
                    if let Some(winit_window) = &window.window {
                        winit_window.set_decorations(decorations);
                    }
                }

                WindowUpdate::SetMousePointer(MousePointer::None) => {
                    if let Some(winit_window) = &window.window {
                        winit_window.set_cursor_visible(false);
                    }
                }

                WindowUpdate::SetMousePointer(MousePointer::SystemDefault) => {
                    if let Some(winit_window) = &window.window {
                        winit_window.set_cursor_visible(true);
                    }
                }
            }
        }

        // If any drawing instructions were taken, then redraw the canvas
        if redraw_canvas {
            window = canvas_drawing.future_desync(move |canvas_drawing| async move {
                if let (Some(winit_window), Some(context), Some(surface), viewport_bounds) = (&window.window, &mut window.context, &mut window.surface, window.viewport_bounds) {
                    // Set up to render at the current size
                    let size    = winit_window.inner_size();
                    let width   = size.width;
                    let height  = size.height;

                    if width != 0 && height != 0 {
                        // Resize the surface before rendering
                        if surface.resize(NonZeroU32::new(width).unwrap(), NonZeroU32::new(height).unwrap()).is_ok() {
                            // Render the region from the canvas drawing
                            let mut buffer              = surface.buffer_mut().unwrap();
                            let buffer_u32: &mut [u32]  = &mut *buffer;
                            let mut frame               = FrameU32Argb::from_u32(width as _, height as _, 2.2, buffer_u32).unwrap();

                            let mut renderer = CanvasDrawingRegionRenderer::new(ShardScanPlanner::default(), ScanlineRenderer::new(canvas_drawing.program_runner(height as _)), height as _);

                            // Set the renderer scaling to match the requested viewport bounds
                            match viewport_bounds {
                                ViewportBounds::All                                 => { }
                                ViewportBounds::Width(requested_width)              => { renderer.viewport_fit_width(&canvas_drawing, width as _, requested_width as _); }
                                ViewportBounds::CenterRegion((x1, y1), (x2, y2))    => { renderer.viewport_fit_center(&canvas_drawing, width as _, (x1 as _)..(x2 as _), (y1 as _)..(y2 as _)); }
                                ViewportBounds::FitExact((x1, y1), (x2, y2))        => { renderer.viewport_fit_exact(&canvas_drawing, width as _, (x1 as _)..(x2 as _), (y1 as _)..(y2 as _)); }
                            }

                            frame.render(renderer, &canvas_drawing);

                            // Present the rendering
                            buffer.present().unwrap();
                        }
                    }
                }

                window
            }.boxed()).await.unwrap();
        }

        // TODO: follow_mouse showing some render glitching at certain points

        // If the transform changed while we were rendering, update the transform between window coordinates and canvas coordinates
        if update_canvas_transform {
            // The inverse transform transforms from the -1,1 regime to canvas coordinates
            let inverse_transform = active_transform.invert();

            if let (Some(window), Some(inverse_active_transform)) = (&window.window, inverse_transform) {
                // Get the size of the window
                let window_size     = window.inner_size();
                let window_scale    = window.scale_factor() as f32;
                let width           = (window_size.width as f32) / (window_scale as f32);
                let height          = (window_size.height as f32) / (window_scale as f32);

                // We scale according to the height
                // TODO: ... also the viewport bounds, which are currently not taken into consideration here
                let scale = 2.0/(height as f32);
                let ratio = (width as f32)/(height as f32);

                // Transform goes between window coordinates and canvas coordinates
                let transform = Transform2D::translate(-ratio, -1.0) * Transform2D::scale(scale / window_scale, scale / window_scale);
                let transform = inverse_active_transform * transform;

                // Send as an event
                events.publish(DrawEvent::CanvasTransform(transform)).await;
            }
        }

        // If any rendering occurred in the last batch of events, send the new frame result
        if send_new_frame {
            // We only send one of these per batch, in case multiple frames are displayed during one batch of events for any reason (this reduces the
            // amount that the rendering can get behind)
            events.publish(DrawEvent::NewFrame).await;

            // Yield control to ensure that other events have a chance to be processed
            let mut yielded = false;
            future::poll_fn(move |context| {
                if !yielded {
                    yielded = true;
                    context.waker().clone().wake();
                    Poll::Pending
                } else {
                    Poll::Ready(())
                }
            }).await;
        }
    }

    // Window will close once the render actions are finished as we drop it here
}

///
/// The list of update events that can occur to a window
///
enum WindowUpdate {
    Draw(Arc<Vec<Draw>>),
    SetTitle(String),
    SetSize((u64, u64)),
    SetFullscreen(bool),
    SetHasDecorations(bool),
    SetMousePointer(MousePointer),
    SetViewportBounds(ViewportBounds),
}

impl fmt::Debug for WindowUpdate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::WindowUpdate::*;

        match self {
            Draw(actions)               => write!(f, "Draw({} actions)", actions.len()),
            SetTitle(title)             => write!(f, "SetTitle({})", title),
            SetSize(sz)                 => write!(f, "SetSize({:?})", sz),
            SetFullscreen(val)          => write!(f, "SetFullscreen({:?})", val),
            SetHasDecorations(val)      => write!(f, "SetHasDecorations({:?})", val),
            SetMousePointer(ptr)        => write!(f, "SetMousePointer({:?})", ptr),
            SetViewportBounds(bounds)   => write!(f, "SetViewportBounds({:?})", bounds),
        }
    }
}

///
/// Stream that merges the streams from the window properties and the renderer into a single stream
///
struct WindowUpdateStream<TDrawStream, TTitleStream, TSizeStream, TFullscreenStream, TDecorationStream, TMousePointerStream, TViewportBoundsStream> {
    draw_stream:        TDrawStream,
    title_stream:       TTitleStream,
    size:               TSizeStream,
    fullscreen:         TFullscreenStream,
    has_decorations:    TDecorationStream,
    mouse_pointer:      TMousePointerStream,
    viewport_bounds:    TViewportBoundsStream,
}

impl<TDrawStream, TTitleStream, TSizeStream, TFullscreenStream, TDecorationStream, TMousePointerStream, TViewportBoundsStream> Stream for WindowUpdateStream<TDrawStream, TTitleStream, TSizeStream, TFullscreenStream, TDecorationStream, TMousePointerStream, TViewportBoundsStream>
where
    TDrawStream:            Unpin + Stream<Item=Arc<Vec<Draw>>>,
    TTitleStream:           Unpin + Stream<Item=String>,
    TSizeStream:            Unpin + Stream<Item=(u64, u64)>,
    TFullscreenStream:      Unpin + Stream<Item=bool>,
    TDecorationStream:      Unpin + Stream<Item=bool>,
    TMousePointerStream:    Unpin + Stream<Item=MousePointer>,
    TViewportBoundsStream:  Unpin + Stream<Item=ViewportBounds>,
{
    type Item = WindowUpdate;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Poll each stream in turn to see if they have an item

        // Rendering instructions have priority
        match self.draw_stream.poll_next_unpin(context) {
            Poll::Ready(Some(item)) => { return Poll::Ready(Some(WindowUpdate::Draw(item))); }
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

        match self.viewport_bounds.poll_next_unpin(context) {
            Poll::Ready(Some(item)) => { return Poll::Ready(Some(WindowUpdate::SetViewportBounds(item))); }
            Poll::Ready(None)       => { return Poll::Ready(None); }
            Poll::Pending           => { }
        }

        // No stream matched anything
        Poll::Pending
    }
}
