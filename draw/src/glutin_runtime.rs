use super::glutin_window::*;
use super::glutin_thread::*;
use super::glutin_thread_event::*;

use glutin::event::{Event};
use glutin::event_loop::{ControlFlow, EventLoopWindowTarget};
use glutin::window::{WindowId};
use futures::task;
use futures::prelude::*;
use futures::future::{LocalBoxFuture};

use std::sync::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::collections::{HashMap};

static NEXT_FUTURE_ID: AtomicU64 = AtomicU64::new(0);

///
/// Represents the state of the Glutin runtime
///
pub (super) struct GlutinRuntime {
    /// The state of the windows being managed by the runtime
    pub (super) windows: HashMap<WindowId, GlutinWindow>,

    /// Maps future IDs to running futures
    pub (super) futures: HashMap<u64, LocalBoxFuture<'static, ()>>
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
    pub fn handle_event(&mut self, event: Event<'_, GlutinThreadEvent>, window_target: &EventLoopWindowTarget<GlutinThreadEvent>, control_flow: &ControlFlow) {
        use Event::*;

        match event {
            NewEvents(_cause)                       => { }
            WindowEvent { window_id: _, event: _ }  => { }
            DeviceEvent { device_id: _, event: _ }  => { }
            UserEvent(thread_event)                 => { self.handle_thread_event(thread_event, window_target, control_flow); }
            Suspended                               => { }
            Resumed                                 => { }
            MainEventsCleared                       => { }
            RedrawRequested(_window_id)             => { }
            RedrawEventsCleared                     => { }
            LoopDestroyed                           => { }
        }
    }

    ///
    /// Handles one of our user events from the GlutinThreadEvent enum
    ///
    fn handle_thread_event(&mut self, event: GlutinThreadEvent, window_target: &EventLoopWindowTarget<GlutinThreadEvent>, control_flow: &ControlFlow) {
        use GlutinThreadEvent::*;

        match event {
            CreateRenderWindow(actions, events) => {
                // Create a window
                let window_builder      = glutin::window::WindowBuilder::new()
                    .with_title("flo_draw")
                    .with_inner_size(glutin::dpi::LogicalSize::new(1024.0, 768.0));
                let windowed_context    = glutin::ContextBuilder::new()
                    .build_windowed(window_builder, &window_target)
                    .unwrap();

                // Store the window context in a new glutin window
                let window_id   = windowed_context.window().id();
                let window      = GlutinWindow::new(windowed_context);

                self.windows.insert(window_id, window);
            },

            RunProcess(start_process) => {
                self.run_process(start_process());
            },

            WakeFuture(future_id) => {
                self.poll_future(future_id);
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
            if let task::Poll::Ready(result) = poll_result {
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
