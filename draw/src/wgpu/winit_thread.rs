use super::winit_runtime::*;
use super::winit_thread_event::*;

use ::desync::*;

use winit::event_loop::{EventLoop, EventLoopProxy};

use std::mem;
use std::sync::*;
use std::sync::mpsc;
use std::thread;
use std::collections::{HashMap};

lazy_static! {
    static ref WINIT_THREAD: Desync<Option<Arc<WinitThread>>> = Desync::new(None);
}

///
/// Represents the thread running the winit event loop
///
pub struct WinitThread {
    event_proxy: Desync<EventLoopProxy<WinitThreadEvent>>
}

impl WinitThread {
    ///
    /// Sends an event to the Winit thread
    ///
    pub fn send_event(&self, event: WinitThreadEvent) {
        self.event_proxy.desync(move |proxy| { proxy.send_event(event).ok(); });
    }
}

///
/// Creates or retrieves the winit thread
///
pub fn winit_thread() -> Arc<WinitThread> {
    WINIT_THREAD.sync(|thread| {
        if let Some(thread) = thread {
            // Thread is already running
            Arc::clone(thread)
        } else {
            // Need to start a new thread
            let new_thread  = create_winit_thread();
            *thread         = Some(Arc::clone(&new_thread));

            new_thread
        }
    })
}

struct StopWinitWhenDropped;
impl Drop for StopWinitWhenDropped {
    fn drop(&mut self) {
        winit_thread().send_event(WinitThreadEvent::StopWhenAllWindowsClosed);
    }
}

///
/// Steals the current thread to run the UI event loop and calls the application function
/// back to continue execution
///
/// This is required because some operating systems (OS X) can't handle UI events from any
/// thread other than the one that's created when the app starts. `flo_draw` will work
/// without this call on operating systems with more robust event handling designs.
///
/// This will also ensure that any graphics are displayed until the user closes the window,
/// which may be useful behaviour even on operating systems where the thread takeover is
/// not required.
///
pub fn with_2d_graphics<TAppFn: 'static+Send+FnOnce() -> ()>(app_fn: TAppFn) {
    // The event loop thread will send us a proxy once it's initialized
    let (send_proxy, recv_proxy) = mpsc::channel();

    // Run the application on a background thread
    thread::Builder::new()
        .name("Application thread".into())
        .spawn(move || {
            WINIT_THREAD.sync(move |thread| {
                // Wait for the proxy to be created
                let proxy = recv_proxy.recv().expect("Winit thread will send us a proxy after initialising");

                // Create the main thread object
                *thread = Some(Arc::new(WinitThread {
                    event_proxy: Desync::new(proxy)
                }));
            });

            // Call back to start the app running
            let stop_winit = StopWinitWhenDropped;

            app_fn();

            mem::drop(stop_winit);
        })
        .expect("Application thread is running");

    // Run the graphics thread on this thread
    run_winit_thread(send_proxy);
}

///
/// Starts the winit thread running
///
fn create_winit_thread() -> Arc<WinitThread> {
    // The event loop thread will send us a proxy once it's initialized
    let (send_proxy, recv_proxy) = mpsc::channel();

    // Run the event loop on its own thread
    thread::Builder::new()
        .name("Winit event thread".into())
        .spawn(move || {
            run_winit_thread(send_proxy)
        })
        .expect("Winit thread is running");

    // Wait for the proxy to be created
    let proxy = recv_proxy.recv().expect("Winit thread will send us a proxy after initialising");

    // Create a WinitThread object to communicate with this thread
    Arc::new(WinitThread {
        event_proxy: Desync::new(proxy)
    })
}

///
/// Runs a winit thread, posting the proxy to the specified channel
///
fn run_winit_thread(send_proxy: mpsc::Sender<EventLoopProxy<WinitThreadEvent>>) {
    // Create the event loop
    let event_loop  = EventLoop::with_user_event();

    // We communicate with the event loop via the proxy
    let proxy       = event_loop.create_proxy();

    // Send the proxy back to the creating thread
    send_proxy.send(proxy).expect("Main thread is waiting to receive its proxy");

    // The runtime struct is used to maintain state when the event loop is running
    let mut runtime = WinitRuntime { 
        window_events:              HashMap::new(),
        pending_redraws:            HashMap::new(),
        futures:                    HashMap::new(),
        pending_yields:             vec![],
        will_stop_when_no_windows:  false,
        will_exit:                  false,
        pointer_id:                 HashMap::new(),
        pointer_state:              HashMap::new(),
    };

    // Run the winit event loop
    event_loop.run(move |event, window_target, control_flow| { 
        runtime.handle_event(event, window_target, control_flow);
    });
}
