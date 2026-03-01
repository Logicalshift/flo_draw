use super::dispatch::*;

use flo_scene::*;

use futures::prelude::*;
use futures::select;
use serde::*;

use tokio::io::unix::AsyncFdReadyGuard;
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
pub fn wayland_event_queue_subprogram<TState>(input: InputStream<WaylandEventQueue<TState>>, context: SceneContext, event_queue: EventQueue<TState>, state: TState) -> impl 'static + Send + Future<Output=()> 
where 
    TState: 'static + FloWaylandState,
{
    let mut input = input;

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

                pending_actions.await;
            }

            // Wait for the fd to become readable (stop on error), and process input while we're waiting
            let mut queue_fd_readable = queue_fd.readable().boxed().fuse();

            let maybe_ready_guard = loop {
                use std::io;
                use std::os::fd::*;

                // We can either receive input from the scene, or the wayland file descriptor can become ready
                enum InputOrReady<'a, TState, TFd> 
                where
                    TFd: AsRawFd,
                {
                    ReadyGuard(io::Result<AsyncFdReadyGuard<'a, TFd>>),
                    Input(Option<WaylandEventQueue<TState>>)
                }

                let next_action = select! {
                    ready_guard = queue_fd_readable             => InputOrReady::ReadyGuard(ready_guard),
                    msg         = input.next().boxed().fuse()   => InputOrReady::Input(msg)
                };

                // If the file descriptor is ready, continue in the outer event handling loop (otherwise handle input events)
                match next_action {
                    // Stop waiting and dispatch pending events if the guard is ready
                    InputOrReady::ReadyGuard(maybe_ready_guard) => { break maybe_ready_guard; }

                    // Handle update requests coming from the scene
                    InputOrReady::Input(Some(WaylandEventQueue::UpdateState(update_state))) => {
                        (update_state)(&mut state);
                    }

                    // Stop processing the event loop if the subprogram is stopped
                    InputOrReady::Input(None) => {
                        return;
                    }
                }
            };
            drop(queue_fd_readable);

            let Ok(mut ready_guard) = maybe_ready_guard else { break; };

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

                    // Other errors probably indicate that the connection has been closed
                    // (Usually a bad file descriptor, which has an uncategorised kind, so we just assume everything means 'wayland is closed')
                    break;
                }

                Err(other) => Err(other),
            }.unwrap();

            // Dispatch pending events (I'm not sure if this is needed because we do this next time through the loop but it's in the example in the wayland-client crate's docs)
            event_queue.dispatch_pending(&mut state).unwrap();

            drop(ready_guard);
        }
    }
}