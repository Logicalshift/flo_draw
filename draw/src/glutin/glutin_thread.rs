use super::glutin_runtime::*;
use super::glutin_thread_event::*;

use ::desync::*;

use winit::event_loop::{EventLoopBuilder, EventLoopProxy};
use once_cell::sync::{Lazy};

use std::mem;
use std::sync::*;
use std::sync::mpsc;
use std::thread;
use std::collections::{HashMap};

static GLUTIN_THREAD: Lazy<Desync<Option<Arc<GlutinThread>>>> = Lazy::new(|| Desync::new(None));

///
/// Represents the thread running the glutin event loop
///
pub struct GlutinThread {
    event_proxy: Desync<EventLoopProxy<GlutinThreadEvent>>
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

struct StopGlutinWhenDropped;
impl Drop for StopGlutinWhenDropped {
    fn drop(&mut self) {
        glutin_thread().send_event(GlutinThreadEvent::StopWhenAllWindowsClosed);
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
            GLUTIN_THREAD.sync(move |thread| {
                // Wait for the proxy to be created
                let proxy = recv_proxy.recv().expect("Glutin thread will send us a proxy after initialising");

                // Create the main thread object
                *thread = Some(Arc::new(GlutinThread {
                    event_proxy: Desync::new(proxy)
                }));
            });

            // Call back to start the app running
            let stop_glutin = StopGlutinWhenDropped;

            app_fn();

            mem::drop(stop_glutin);
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
    let event_loop  = EventLoopBuilder::with_user_event().build().unwrap();

    // We communicate with the event loop via the proxy
    let proxy       = event_loop.create_proxy();

    // Send the proxy back to the creating thread
    send_proxy.send(proxy).expect("Main thread is waiting to receive its proxy");

    // The runtime struct is used to maintain state when the event loop is running
    let mut runtime = GlutinRuntime { 
        window_events:              HashMap::new(),
        futures:                    HashMap::new(),
        will_stop_when_no_windows:  false,
        will_exit:                  false,
        pointer_id:                 HashMap::new(),
        pointer_state:              HashMap::new(),
        suspended:                  true,
    };

    // Run the glutin event loop
    event_loop.run(move |event, window_target| { 
        runtime.handle_event(event, window_target);
    }).unwrap();
}
