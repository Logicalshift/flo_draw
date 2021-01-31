use super::draw_event::*;
use super::glutin_window::*;
use super::glutin_thread::*;
use super::glutin_thread_event::*;

use flo_stream::*;

use glutin::{GlRequest, Api};
use glutin::event::{Event, WindowEvent};
use glutin::event_loop::{ControlFlow, EventLoopWindowTarget};
use glutin::window::{WindowId};
use futures::task;
use futures::prelude::*;
use futures::future::{LocalBoxFuture};

use std::mem;
use std::sync::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::collections::{HashMap};

static NEXT_FUTURE_ID: AtomicU64 = AtomicU64::new(0);

///
/// Represents the state of the Glutin runtime
///
pub (super) struct GlutinRuntime {
    /// The event publishers for the windows being managed by the runtime
    pub (super) window_events: HashMap<WindowId, Publisher<DrawEvent>>,

    /// Maps future IDs to running futures
    pub (super) futures: HashMap<u64, LocalBoxFuture<'static, ()>>,

    /// Set to true if this runtime will stop when all the windows are closed
    pub (super) will_stop_when_no_windows: bool
}

///
/// Used to wake a future running on the glutin thread
///
struct GlutinFutureWaker {
    future_id: u64
}

impl GlutinRuntime {
    ///
    /// Handles an event from the rest of the process and updates the state
    ///
    pub fn handle_event(&mut self, event: Event<'_, GlutinThreadEvent>, window_target: &EventLoopWindowTarget<GlutinThreadEvent>, control_flow: &mut ControlFlow) {
        use Event::*;

        match event {
            NewEvents(_cause)                       => { }
            WindowEvent { window_id, event }        => { self.handle_window_event(window_id, event); }
            DeviceEvent { device_id: _, event: _ }  => { }
            UserEvent(thread_event)                 => { self.handle_thread_event(thread_event, window_target, control_flow); }
            Suspended                               => { }
            Resumed                                 => { }
            MainEventsCleared                       => { }
            RedrawRequested(window_id)              => { self.request_redraw(window_id); }
            RedrawEventsCleared                     => { }
            LoopDestroyed                           => { }
        }
    }

    ///
    /// Handles a glutin window event
    ///
    fn handle_window_event(&mut self, window_id: WindowId, event: WindowEvent) {
        if let Some(window_events) = self.window_events.get_mut(&window_id) {
            use WindowEvent::*;

            // Generate draw_events for the window event
            let draw_events = match event {
                Resized(new_size)                                               => vec![DrawEvent::Resize(new_size.width as f64, new_size.height as f64)],
                Moved(_position)                                                => vec![],
                CloseRequested                                                  => vec![DrawEvent::Closed],
                Destroyed                                                       => vec![],
                DroppedFile(_path)                                              => vec![],
                HoveredFile(_path)                                              => vec![],
                HoveredFileCancelled                                            => vec![],
                ReceivedCharacter(_c)                                           => vec![],
                Focused(_focused)                                               => vec![],
                KeyboardInput { device_id: _, input: _, is_synthetic: _, }      => vec![],
                ModifiersChanged(_state)                                        => vec![],
                CursorMoved { device_id: _, position: _, .. }                   => vec![],
                CursorEntered { device_id: _ }                                  => vec![],
                CursorLeft { device_id: _ }                                     => vec![],
                MouseWheel { device_id: _, delta: _, phase: _, .. }             => vec![],
                MouseInput { device_id: _, state: _, button: _, .. }            => vec![],
                TouchpadPressure { device_id: _, pressure: _, stage: _, }       => vec![],
                AxisMotion { device_id: _, axis: _, value: _ }                  => vec![],
                Touch(_touch)                                                   => vec![],
                ScaleFactorChanged { scale_factor, new_inner_size }             => vec![DrawEvent::Scale(scale_factor), DrawEvent::Resize(new_inner_size.width as f64, new_inner_size.height as f64)],
                ThemeChanged(_theme)                                            => vec![],
            };

            // Dispatch the draw events using a process
            if draw_events.len() > 0 {
                // Need to republish the window events so we can share with the process
                let mut window_events = window_events.republish();

                self.run_process(async move {
                    for evt in draw_events {
                        window_events.publish(evt).await;
                    }
                });
            }
        }
    }

    ///
    /// Sends a redraw request to a window
    ///
    fn request_redraw(&mut self, window_id: WindowId) {
        if let Some(window_events) = self.window_events.get_mut(&window_id) {
            // Need to republish the window events so we can share with the process
            let mut window_events = window_events.republish();

            self.run_process(async move {
                window_events.publish(DrawEvent::Redraw).await;
            });
        }
    }

    ///
    /// Handles one of our user events from the GlutinThreadEvent enum
    ///
    fn handle_thread_event(&mut self, event: GlutinThreadEvent, window_target: &EventLoopWindowTarget<GlutinThreadEvent>, control_flow: &mut ControlFlow) {
        use GlutinThreadEvent::*;

        match event {
            CreateRenderWindow(actions, events) => {
                // Create a window
                let window_builder      = glutin::window::WindowBuilder::new()
                    .with_title("flo_draw")
                    .with_inner_size(glutin::dpi::LogicalSize::new(1024.0, 768.0));
                let windowed_context    = glutin::ContextBuilder::new()
                    .with_gl(GlRequest::Specific(Api::OpenGl, (3, 3)))
                    .build_windowed(window_builder, &window_target)
                    .unwrap();

                // Store the window context in a new glutin window
                let window_id           = windowed_context.window().id();
                let size                = windowed_context.window().inner_size();
                let scale               = windowed_context.window().scale_factor();
                let window              = GlutinWindow::new(windowed_context);

                // Store the publisher for the events for this window
                let mut initial_events  = events.republish();
                self.window_events.insert(window_id, events);

                // Run the window as a process on this thread
                self.run_process(async move { 
                    // Send the initial events for this window (set the size and the DPI)
                    initial_events.publish(DrawEvent::Resize(size.width as f64, size.height as f64)).await;
                    initial_events.publish(DrawEvent::Scale(scale)).await;
                    initial_events.publish(DrawEvent::Redraw).await;
                    mem::drop(initial_events);

                    // Process the actions for the window
                    send_actions_to_window(window, actions).await; 

                    // Stop processing events for the window once there are no more actions
                    glutin_thread().send_event(GlutinThreadEvent::StopSendingToWindow(window_id));
                });
            }

            StopSendingToWindow(window_id) => {
                self.window_events.remove(&window_id);

                if self.window_events.len() == 0 && self.will_stop_when_no_windows {
                    *control_flow = ControlFlow::Exit;
                }
            }

            RunProcess(start_process) => {
                self.run_process(start_process());
            },

            WakeFuture(future_id) => {
                self.poll_future(future_id);
            },

            StopWhenAllWindowsClosed => {
                self.will_stop_when_no_windows = true;

                if self.window_events.len() == 0 {
                    *control_flow = ControlFlow::Exit;
                }
            }
        }
    }

    ///
    /// Runs a process in the context of this runtime
    ///
    fn run_process<Fut: 'static+Future<Output=()>>(&mut self, future: Fut) {
        // Box the future for polling
        let future = future.boxed_local();

        // Assign an ID to this future (we use this for waking it up)
        let future_id = NEXT_FUTURE_ID.fetch_add(1, Ordering::Relaxed);

        // Store in the runtime
        self.futures.insert(future_id, future);

        // Perform the initial polling operation on the future
        self.poll_future(future_id);
    }

    ///
    /// Causes the future with the specified ID to be polled
    ///
    fn poll_future(&mut self, future_id: u64) {
        if let Some(future) = self.futures.get_mut(&future_id) {
            // Create a context to poll this future in
            let glutin_waker        = GlutinFutureWaker { future_id };
            let glutin_waker        = task::waker(Arc::new(glutin_waker));
            let mut glutin_context  = task::Context::from_waker(&glutin_waker);

            // Poll the future
            let poll_result         = future.poll_unpin(&mut glutin_context);

            // Remove the future from the list if it has completed
            if let task::Poll::Ready(_) = poll_result {
                self.futures.remove(&future_id);
            }
        }
    }
}

impl task::ArcWake for GlutinFutureWaker {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        // Send a wake request to glutin
        glutin_thread().send_event(GlutinThreadEvent::WakeFuture(arc_self.future_id));
    }
}
