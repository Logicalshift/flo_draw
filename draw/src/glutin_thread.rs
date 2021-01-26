use super::glutin_window::*;
use super::glutin_thread_event::*;

use ::desync::*;

use glutin::{ContextBuilder};
use glutin::event::{Event};
use glutin::event_loop::{ControlFlow, EventLoop, EventLoopProxy, EventLoopWindowTarget};
use glutin::window::{WindowId, WindowBuilder};
use futures::task;
use futures::prelude::*;
use futures::future::{LocalBoxFuture};

use std::sync::*;
use std::sync::mpsc;
use std::thread;
use std::sync::atomic::{AtomicU64, Ordering};
use std::collections::{HashMap};

lazy_static! {
    static ref GLUTIN_THREAD: Desync<Option<Arc<GlutinThread>>> = Desync::new(None);
}

static NEXT_FUTURE_ID: AtomicU64 = AtomicU64::new(0);

///
/// Represents the thread running the glutin event loop
///
pub struct GlutinThread {
    event_proxy: Desync<EventLoopProxy<GlutinThreadEvent>>
}

///
/// Represents the state of the Glutin runtime
///
struct GlutinRuntime {
    /// The state of the windows being managed by the runtime
    windows: HashMap<WindowId, GlutinWindow>,

    /// Maps future IDs to running futures
    futures: HashMap<u64, LocalBoxFuture<'static, ()>>
}

///
/// Used to wake a future running on the glutin thread
///
struct GlutinFutureWaker {
    future_id: u64
}

impl GlutinThread {
    ///
    /// Sends an event to the Glutin thread
    ///
    pub fn send_event(&self, event: GlutinThreadEvent) {
        self.event_proxy.desync(move |proxy| { proxy.send_event(event).ok(); });
    }
}

///
/// Creates or retrieves the glutin thread
///
pub fn glutin_thread() -> Arc<GlutinThread> {
    GLUTIN_THREAD.sync(|thread| {
        if let Some(thread) = thread {
            // Thread is already running
            Arc::clone(thread)
        } else {
            // Need to start a new thread
            let new_thread  = create_glutin_thread();
            *thread         = Some(Arc::clone(&new_thread));

            new_thread
        }
    })
}

///
/// Steals the current thread to run the UI event loop and calls the application function
/// back to continue execution
///
/// This is required because some operating systems (OS X) can't handle UI events from any
/// thread other than the one that's created when the app starts. `flo_draw` will work
/// without this call on operating systems with more robust event handling designs.
///
pub fn with_2d_graphics<TAppFn: 'static+Send+FnOnce() -> ()>(app_fn: TAppFn) {
    // The event loop thread will send us a proxy once it's initialized
    let (send_proxy, recv_proxy) = mpsc::channel();

    // Run the application on a background thread
    thread::Builder::new()
        .name("Application thread".into())
        .spawn(move || {
            GLUTIN_THREAD.sync(move |thread| {
                // Wait for the proxy to be created
                let proxy = recv_proxy.recv().expect("Glutin thread will send us a proxy after initialising");

                // Create the main thread object
                *thread = Some(Arc::new(GlutinThread {
                    event_proxy: Desync::new(proxy)
                }));
            });

            // Call back to start the app running
            app_fn();
        })
        .expect("Application thread is running");

    // Run the graphics thread on this thread
    run_glutin_thread(send_proxy);
}

///
/// Starts the glutin thread running
///
fn create_glutin_thread() -> Arc<GlutinThread> {
    // The event loop thread will send us a proxy once it's initialized
    let (send_proxy, recv_proxy) = mpsc::channel();

    // Run the event loop on its own thread
    thread::Builder::new()
        .name("Glutin event thread".into())
        .spawn(move || {
            run_glutin_thread(send_proxy)
        })
        .expect("Glutin thread is running");

    // Wait for the proxy to be created
    let proxy = recv_proxy.recv().expect("Glutin thread will send us a proxy after initialising");

    // Create a GlutinThread object to communicate with this thread
    Arc::new(GlutinThread {
        event_proxy: Desync::new(proxy)
    })
}

///
/// Runs a glutin thread, posting the proxy to the specified channel
///
fn run_glutin_thread(send_proxy: mpsc::Sender<EventLoopProxy<GlutinThreadEvent>>) {
    // Create the event loop
    let event_loop  = EventLoop::with_user_event();

    // We communicate with the event loop via the proxy
    let proxy       = event_loop.create_proxy();

    // Send the proxy back to the creating thread
    send_proxy.send(proxy).expect("Main thread is waiting to receive its proxy");

    // The runtime struct is used to maintain state when the event loop is running
    let mut runtime = GlutinRuntime { 
        windows: HashMap::new(),
        futures: HashMap::new()
    };

    // Run the event loop
    event_loop.run(move |event, window_target, control_flow| { 
        runtime.handle_event(event, window_target, control_flow);
    });
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
