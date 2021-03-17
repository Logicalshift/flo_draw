use super::draw::*;

use std::collections::vec_deque::*;
use std::sync::*;
use std::pin::*;

use futures::task;
use futures::task::{Poll, Waker};
use futures::{Stream};

///
/// Internals of a canvas stream
///
struct CanvasStreamCore {
    /// Items waiting to be drawn for this stream
    queue: VecDeque<Draw>,

    /// The task to notify when extra data is available
    waiting_task: Option<Waker>,

    /// Set to true when the canvas is dropped
    canvas_dropped: bool,

    /// Set to true if the stream is dropped
    stream_dropped: bool
}

///
/// The canvas stream can be used to read the contents of the canvas and follow new content as it arrives.
/// Unconsumed commands are cut off if the `Draw::ClearCanvas` command is issued.
///
pub (crate) struct CanvasStream {
    /// The core of this stream
    core: Mutex<CanvasStreamCore>
}

impl CanvasStream {
    ///
    /// Creates a new canvas stream
    ///
    pub fn new() -> CanvasStream {
        CanvasStream {
            core: Mutex::new(CanvasStreamCore {
                queue:          VecDeque::new(),
                waiting_task:   None,
                canvas_dropped: false,
                stream_dropped: false
            })
        }
    }

    ///
    /// Wakes the stream task
    ///
    pub (crate) fn notify_dropped(&self) {
        let mut core = self.core.lock().unwrap();

        core.canvas_dropped = true;

        if let Some(task) = core.waiting_task.take() {
            task.wake();
        }
    }

    ///
    /// Sends some drawing commands to this stream (returning true if this stream wants more commands)
    ///
    pub (crate) fn send_drawing<DrawIter: Iterator<Item=Draw>> (&self, drawing: DrawIter, clear_pending: bool) -> bool {
        let mut core = self.core.lock().unwrap();

        // Clear out any pending commands if they're hidden by a clear (except frame commands, which we need to add up)
        if clear_pending {
            core.queue.retain(|action| {
                match action {
                    Draw::StartFrame    |
                    Draw::ShowFrame     |
                    Draw::ResetFrame    => true,
                    _                   => false
                }
            });
        }

        // Push the drawing commands
        for draw in drawing {
            core.queue.push_back(draw);
        }

        // If a task needs waking up, wake it
        if let Some(task) = core.waiting_task.take() {
            task.wake();
        }

        // We want more commands if the stream is not dropped
        !core.stream_dropped
    }
}

impl CanvasStream {
    fn poll(&self, context: &task::Context) -> Poll<Option<Draw>> {
        let mut core = self.core.lock().unwrap();

        if let Some(value) = core.queue.pop_front() {
            Poll::Ready(Some(value))
        } else if core.canvas_dropped {
            Poll::Ready(None)
        } else {
            core.waiting_task = Some(context.waker().clone());
            Poll::Pending
        }
   }
}

///
/// The 'fragile' canvas stream is a variant of the canvas stream that marks the
/// stream as being dropped if this happens (so that we can remove it from the
/// list in the canvas)
///
pub (crate) struct FragileCanvasStream {
    stream: Arc<CanvasStream>
}

impl FragileCanvasStream {
    pub fn new(stream: Arc<CanvasStream>) -> FragileCanvasStream {
        FragileCanvasStream { stream: stream }
    }
}

impl Drop for FragileCanvasStream {
    fn drop(&mut self) {
        let mut core = self.stream.core.lock().unwrap();

        core.stream_dropped = true;
    }
}

impl Stream for FragileCanvasStream {
    type Item = Draw;

    fn poll_next(self: Pin<&mut Self>, context: &mut task::Context) -> Poll<Option<Draw>> {
        self.stream.poll(context)
    }
}
