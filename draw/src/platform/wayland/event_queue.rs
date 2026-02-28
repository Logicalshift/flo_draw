use flo_scene::*;

use futures::prelude::*;
use serde::*;

use tokio::io::unix::{AsyncFd};

use wayland_backend::client::{WaylandError};
use wayland_client::{EventQueue};

use std::os::fd::{AsFd};

///
/// Messages that can be sent to control a Wayland event queue
///
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WaylandEventQueue {

}

impl SceneMessage for WaylandEventQueue {
}

///
/// Subprogram that runs a wayland event queue
///
pub async fn wayland_event_queue_subprogram<TState>(input: InputStream<WaylandEventQueue>, context: SceneContext, event_queue: EventQueue<TState>, state: TState)
where 
    TState: 'static + Send,
{
    // Create the future that runs the event queue
    let event_queue_future = run_event_queue(event_queue, state);

    // Also process the input events
    let input_events_future = async move {
        let mut input = input;
        while let Some(_msg) = input.next().await {
        }
    };

    future::select(event_queue_future.boxed(), input_events_future.boxed()).await;
}

///
/// Future that runs the event queue
///
fn run_event_queue<TState>(event_queue: EventQueue<TState>, state: TState) -> impl 'static + Send + Future<Output=()> 
where
    TState: 'static + Send,
{
    async move {
        let mut state       = state;
        let mut event_queue = event_queue;

        // Fetch the filedescriptor for the event queue
        let queue_fd = event_queue.as_fd();
        let queue_fd = queue_fd.try_clone_to_owned().unwrap();

        // Create an AsyncFd from the file descriptor
        let queue_fd = AsyncFd::new(queue_fd).unwrap();

        loop {
            // Flush the queue and dispatch any pending events
            event_queue.flush().unwrap();
            event_queue.dispatch_pending(&mut state).unwrap();

            // Wait for the fd to become readable (stop on error)
            let Ok(ready_guard) = queue_fd.readable().await else { break; };

            // Acquire the read guard (need to call dispatch_pending if it returns 'None', which we can do by just continuing the loop)
            let Some(guard) = event_queue.prepare_read() else { continue; };
            match guard.read() {
                Ok(_) => { Ok(()) }
                Err(WaylandError::Io(io_error)) => {
                    use std::io::*;

                    if io_error.kind() == ErrorKind::WouldBlock {
                        // WouldBlock might indicate a race condition with another thread
                        continue;
                    }

                    Err(WaylandError::Io(io_error))
                }

                Err(other) => Err(other),
            }.unwrap();

            // Dispatch pending events (I'm not sure if this is needed because we do this next time through the loop but it's in the example in the wayland-client crate's docs)
            event_queue.dispatch_pending(&mut state).unwrap();

            drop(ready_guard);
        }
    }
}