use crate::draw::*;
use crate::draw_resource::*;

use ::desync::*;
use futures::task;
use futures::task::{Poll, Waker};
use futures::prelude::*;

use std::mem;
use std::pin::*;
use std::sync::*;
use std::collections::{VecDeque, HashSet, HashMap};

///
/// The draw stream core contains the shared data structures for a stream of drawing instructions
///
pub (crate) struct DrawStreamCore {
    /// The pending drawing instructions, and the resource that it affects
    pending_drawing: Vec<(DrawResource, Draw)>,

    /// The resource that the stream is currently drawing to
    target_resource: DrawResource,

    /// Once the pending drawing has been cleared, the stream should be considered as 'closed'
    closed: bool,

    /// The waker for this core, if there is one
    waiting_task: Option<Waker>
}

///
/// A draw stream relays `Draw` instructions from a source such as a `Canvas` or a `DrawContext` as a stream
///
pub struct DrawStream {
    /// The core of this draw stream (queues up pending drawing instructions)
    core: Arc<Desync<DrawStreamCore>>,

    /// Drawing instructions buffered from the core
    buffer: VecDeque<Draw>
}

impl DrawStreamCore {
    ///
    /// Creates a new stream core
    ///
    pub fn new() -> DrawStreamCore {
        // No drawing instructions, and drawing to layer 0 by default
        DrawStreamCore {
            pending_drawing:    vec![],
            target_resource:    DrawResource::Layer(LayerId(0)),
            closed:             false,
            waiting_task:       None
        }
    }

    ///
    /// Removes all references that change the specified resource
    ///
    pub fn clear_resource(&mut self, resource: DrawResource) {
        self.pending_drawing.retain(|(drawing_resource, _)| drawing_resource != &resource);
    }

    ///
    /// Removes any resource in this stream that's declared but not used
    ///
    pub fn remove_unused_resources(&mut self) {
        let mut unused_resources    = HashMap::new();
        let mut to_remove           = HashSet::new();

        for (idx, (target_resource, draw)) in self.pending_drawing.iter().enumerate() {
            // Figure out the resources used by this step
            let used_resources = draw.source_resource(target_resource);

            // If no resources are used, then this is declaring a new resource
            if used_resources.len() == 0 {
                // If the target resource is in the unused list, remove any item that refers to it
                if let Some(declaration_indexes) = unused_resources.remove(target_resource) {
                    to_remove.extend(declaration_indexes);
                }

                // This is declaring this resource: add it as 'unused'
                unused_resources.insert(*target_resource, vec![idx]);
            } else {
                // Remove the used resources from the unused list
                for resource in used_resources {
                    // A self-reference doesn't count as a usage of the resource (just another spot where it's unused)
                    if &resource != target_resource {
                        unused_resources.remove(&resource);
                    } else {
                        // A self-reference is added to the indexes that form the declaration of the resource (except for layers and sprites)
                        match resource {
                            DrawResource::Layer(_) | DrawResource::Sprite(_)    => { },
                            _                                                   => { unused_resources.get_mut(&resource).map(|declaration_list| declaration_list.push(idx)); }
                        }
                    }
                }
            }
        }

        // Remove any resources in the to_remove set
        if to_remove.len() > 0 {
            let old_drawing         = mem::take(&mut self.pending_drawing);
            self.pending_drawing    = old_drawing.into_iter()
                .enumerate()
                .filter(|(idx, _item)| !to_remove.contains(idx))
                .map(|(_idx, item)| item)
                .collect();
        }
    }

    ///
    /// Writes a stream of instructions to this drawing stream
    ///
    pub fn write<DrawIter: Iterator<Item=Draw>>(&mut self, drawing: DrawIter) {
        let mut drawing_cleared = false;

        for draw in drawing {
            // Process the drawing instruction
            match &draw {
                Draw::Layer(layer_id)   => { self.target_resource = DrawResource::Layer(*layer_id); },
                Draw::Sprite(sprite_id) => { self.target_resource = DrawResource::Sprite(*sprite_id); },

                Draw::ClearLayer        => { self.clear_resource(self.target_resource); drawing_cleared = true; },
                Draw::ClearSprite       => { self.clear_resource(self.target_resource); drawing_cleared = true; },

                Draw::ClearCanvas(_)    => { 
                    self.pending_drawing = vec![];
                    self.target_resource = DrawResource::Layer(LayerId(0));
                },

                _                       => { }
            }

            // Add to the pending drawing
            let drawing_target = draw.target_resource(&self.target_resource);
            self.pending_drawing.push((drawing_target, draw));
        }

        // If we've processed a clear instruction, clear out any unused resources from the pending list
        if drawing_cleared {
            self.remove_unused_resources();
        }
    }

    ///
    /// Marks this core as closed
    ///
    pub fn close(&mut self) {
        self.closed = true;
    }

    ///
    /// Returns the waker for anything listening for changes to the stream
    ///
    pub fn take_waker(&mut self) -> Option<Waker> {
        self.waiting_task.take()
    }
}

impl DrawStream {
    ///
    /// Creates a draw stream that will read from the specified core
    ///
    pub (crate) fn with_core(core: &Arc<Desync<DrawStreamCore>>) -> DrawStream {
        DrawStream {
            core:   Arc::clone(core),
            buffer: VecDeque::new()
        }
    }
}

impl Stream for DrawStream {
    type Item = Draw;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut task::Context) -> Poll<Option<Draw>> {
        // Read from the buffer if there are any items waiting
        if self.buffer.len() > 0 {
            // Read from the front of the buffer
            Poll::Ready(self.buffer.pop_front())
        } else {
            // Attempt to load the buffer from the core. If it's still empty, create a notification
            let (new_buffer, closed) = self.core.sync(|core| {
                if core.pending_drawing.len() == 0 {
                    // No drawing is waiting, so set the task and return an empty buffer (will be no items in the result)
                    core.waiting_task = Some(context.waker().clone());

                    (VecDeque::new(), core.closed)
                } else {
                    // Convert the buffer for reading (will always be at least one item in the result)
                    let new_buffer = core.pending_drawing.drain(..)
                        .map(|(_, draw)| draw)
                        .collect();

                    (new_buffer, core.closed)
                }
            });

            self.buffer = new_buffer;

            if self.buffer.len() > 0 {
                // Read from the front of the buffer
                Poll::Ready(self.buffer.pop_front())
            } else if closed {
                // No data to read and the core is marked as closed
                Poll::Ready(None)
            } else {
                // No data to read and the waker is set
                Poll::Pending
            }
        }
    }
}
