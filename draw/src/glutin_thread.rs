use super::glutin_thread_event::*;

use ::desync::*;

use glutin::event_loop::{EventLoop, EventLoopProxy};

use std::sync::*;
use std::sync::mpsc;
use std::thread;

lazy_static! {
    static ref GLUTIN_THREAD: Desync<Option<Arc<GlutinThread>>> = Desync::new(None);
}

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
            // Create the event loop
            let event_loop  = EventLoop::with_user_event();

            // We communicate with the event loop via the proxy
            let proxy       = event_loop.create_proxy();

            // Send the proxy back to the creating thread
            send_proxy.send(proxy).expect("Main thread is waiting to receive its proxy");

            // Run the event loop
            event_loop.run(|_event, _window_target, _control_flow| { });
        })
        .expect("Glutin thread is running");

    // Wait for the proxy to be created
    let proxy = recv_proxy.recv().expect("Glutin thread will send us a proxy after initialising");

    // Create a GlutinThread object to communicate with this thread
    Arc::new(GlutinThread {
        event_proxy: Desync::new(proxy)
    })
}
