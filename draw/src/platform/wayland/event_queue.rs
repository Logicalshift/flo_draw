use super::dispatch::*;

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
pub enum WaylandEventQueue<TState> {
    /// Updates the state for this event queue
    UpdateState(Box<dyn Send + FnOnce(&mut TState)>),
}

///
/// Trait implemented by event queue state objects that are scheduled in the scene
///
pub trait FloWaylandState : Send {
    ///
    /// If this state dispatches events, this retrieves the dispatcher
    ///
    fn dispatcher(&mut self) -> Option<&mut FloWaylandDispatcher>;
}

impl<TState> SceneMessage for WaylandEventQueue<TState> 
where
    TState: 'static + Send
{
    #[inline] fn serializable() -> bool { false }
}

impl<TState> Serialize for WaylandEventQueue<TState> {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer 
    {
        use serde::ser::{Error};
        Err(S::Error::custom("WaylandEventQueue cannot be serialized"))
    }
}

impl<'a, TState> Deserialize<'a> for WaylandEventQueue<TState> {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'a> 
    {
        use serde::de::{Error};
        Err(D::Error::custom("WaylandEventQueue cannot be serialized"))
    }
}

///
/// Subprogram that runs a wayland event queue
///
pub async fn wayland_event_queue_subprogram<TState>(input: InputStream<WaylandEventQueue<TState>>, context: SceneContext, event_queue: EventQueue<TState>, state: TState)
where 
    TState: 'static + FloWaylandState,
{
    // Create the future that runs the event queue
    let event_queue_future = run_event_queue(event_queue, state, context);

    // Also process the input events
    let input_events_future = async move {
        let mut input = input;
        while let Some(msg) = input.next().await {
            match msg {
                WaylandEventQueue::UpdateState(update_fn) => {
                    // Call the function back on the current state stored with the event queue
                    //let mut state = state.lock().unwrap();
                    //(update_fn)(&mut *state);
                }
            }
        }
    };

    future::select(event_queue_future.boxed(), input_events_future.boxed()).await;
}

///
/// Future that runs the event queue
///
fn run_event_queue<TState>(event_queue: EventQueue<TState>, state: TState, context: SceneContext) -> impl 'static + Send + Future<Output=()> 
where
    TState: 'static + FloWaylandState,
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
            let Ok(_) = event_queue.flush() else { break; };
            let Ok(_) = event_queue.dispatch_pending(&mut state) else { break; };

            // Dispatch any pending actions
            if let Some(dispatcher) = state.dispatcher() {
                let pending_actions = FloWaylandDispatcher::execute_pending(dispatcher, &context);
                drop(dispatcher);

                pending_actions.await;
            }

            // Wait for the fd to become readable (stop on error)
            let Ok(mut ready_guard) = queue_fd.readable().await else { break; };

            // Acquire the read guard (need to call dispatch_pending if it returns 'None', which we can do by just continuing the loop)
            let Some(guard) = event_queue.prepare_read() else { continue; };
            match guard.read() {
                Ok(_) => { Ok(()) }
                Err(WaylandError::Io(io_error)) => {
                    use std::io::*;

                    if io_error.kind() == ErrorKind::WouldBlock {
                        // WouldBlock indicates a spurious wakeup
                        ready_guard.clear_ready();
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