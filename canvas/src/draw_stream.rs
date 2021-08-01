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

    /// The number of writers that this stream core has
    usage_count: usize,

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
            usage_count:        0,
            closed:             false,
            waiting_task:       None
        }
    }

    ///
    /// Increases the usage count of this core
    ///
    pub fn add_usage(&mut self) {
        self.usage_count += 1;
    }

    ///
    /// Decreases the usage count of this core and returns the new usage count
    ///
    pub fn finish_usage(&mut self) -> usize {
        if self.usage_count > 0 {
            self.usage_count -= 1;
        }

        self.usage_count
    }

    ///
    /// On restore, rewinds the canvas to before the last store operation
    ///
    pub fn rewind_to_last_store(&mut self) {
        let mut last_store = None;

        // Search backwards in the drawing commands for the last store command
        let mut state_stack_depth = 0;

        for draw_index in (0..self.pending_drawing.len()).rev() {
            match self.pending_drawing[draw_index] {
                // Commands that might cause the store/restore to not undo perfectly break the sequence
                (_, Draw::Clip)         |
                (_, Draw::Unclip)       |
                (_, Draw::StartFrame)   |
                (_, Draw::ShowFrame)    => break,

                // If the state stack has a pop for every push then we can remove these requests too
                // TODO: this has a bug in that if the final event is a 'push' instead of a 'pop'
                // then it will mistakenly believe the states can be removed
                (_, Draw::PushState)    => { state_stack_depth += 1; },
                (_, Draw::PopState)     => { state_stack_depth -= 1; },

                // If we find no sequence breaks and a store, this is where we want to rewind to
                (_, Draw::Store)        => {
                    if state_stack_depth == 0 {
                        last_store = Some(draw_index+1);
                    }
                    break;
                },

                _               => ()
            };
        }

        // Remove everything up to the last store position
        if let Some(last_store) = last_store {
            self.pending_drawing.truncate(last_store);
        }
    }

    ///
    /// Removes any 'start frame' that's matched by a 'show frame' or a 'reset frame'
    ///
    pub fn balance_show_frames(&mut self) {
        // The indexes of instructions that cancel each other out
        let mut indexes_to_remove   = vec![];

        // The stack of active 'show frame' instructions
        let mut frame_stack         = vec![];

        // Whether or not the pending drawing should be rewritten in a 'start frame/show frame' pair and if it should start by resetting the frame
        let mut show_frame          = false;
        let mut reset_frame         = false;

        // Trace the pending drawings for frame operations
        for (idx, (_resource, draw)) in self.pending_drawing.iter().enumerate() {
            match draw {
                Draw::StartFrame    => frame_stack.push(idx),
                Draw::ShowFrame     => {
                    // Remove the corresponding 'start frame' instruction
                    if let Some(start_frame_idx) = frame_stack.pop() {
                        show_frame = true;

                        indexes_to_remove.push(start_frame_idx);
                        indexes_to_remove.push(idx);
                    }
                }

                Draw::ResetFrame    => {
                    // We move the 'reset frame' request to the start of the stack
                    reset_frame = true;
    
                    // Remove all frame instructions up to this point
                    indexes_to_remove.extend(frame_stack.drain(..));
                    indexes_to_remove.push(idx);
                }

                _                   => { }
            }
        }

        // Some patterns are 'already balanced' and don't require us to rewrite the whole list of drawing instructions
        let already_balanced = match (show_frame, reset_frame) {
            (false, false)  => vec![],
            (false, true)   => vec![0],
            (true, false)   => vec![0, self.pending_drawing.len()-1],
            (true, true)    => vec![0, 1, self.pending_drawing.len()-1]
        };

        if indexes_to_remove == already_balanced {
            // If there's just a 'start frame' and a 'show frame' at the end, then there's nothing to do
        } else if indexes_to_remove.len() > 0 {
            // Rewrite the pending queue if there was anything to remove
            let indexes_to_remove   = indexes_to_remove.into_iter().collect::<HashSet<_>>();
            let old_drawing         = mem::take(&mut self.pending_drawing);
            self.pending_drawing    = old_drawing.into_iter()
                .enumerate()
                .filter(|(idx, _item)| !indexes_to_remove.contains(idx))
                .map(|(_idx, item)| item)
                .collect();

            // If any frame instructions were used, wrap the whole set of instructions in a frame (reset frames move to the start)
            if show_frame {
                self.pending_drawing.insert(0, (DrawResource::Frame, Draw::StartFrame));
                self.pending_drawing.push((DrawResource::Frame, Draw::ShowFrame));
            }

            if reset_frame {
                self.pending_drawing.insert(0, (DrawResource::Frame, Draw::ResetFrame));
            }
        }
    }

    ///
    /// Removes all references that change the specified resource
    ///
    pub fn clear_resource(&mut self, resource: DrawResource) {
        // The indexes that are unused
        let mut unused_indexes      = HashSet::new();

        // The indexes that we're tracking as unused
        let mut maybe_unused        = vec![];

        // The index where the resource was last selected
        let mut last_selection_idx  = None;

        // Analyse the pending drawing for any place the resource is targeted, and for any place it's used
        for (idx, (target_resource, draw)) in self.pending_drawing.iter().enumerate() {
            match draw {
                Draw::Sprite(sprite_id) => { if resource == DrawResource::Sprite(*sprite_id)    { last_selection_idx = Some(idx); } },
                Draw::Layer(layer_id)   => { if resource == DrawResource::Layer(*layer_id)      { last_selection_idx = Some(idx); } }

                _ => {}
            }

            if target_resource == &resource {
                // If this re-declares this resource, then none of the 'maybe unused' indexes are actually unused
                if draw.source_resource(target_resource).len() == 0 {
                    unused_indexes.extend(maybe_unused.drain(..));
                }

                // Add to the maybe unused list
                maybe_unused.push(idx);
            } else if draw.uses_resource(&resource) {
                // If the resource is used, then these indexes and the last selection index should not be cleared
                last_selection_idx.take().map(|idx| unused_indexes.remove(&idx));
                maybe_unused = vec![];
            }
        }

        // Anything that hasn't been used yet won't be used
        unused_indexes.extend(maybe_unused);

        // Remove any item in the unused index list
        if unused_indexes.len() > 0 {
            let old_drawing         = mem::take(&mut self.pending_drawing);
            self.pending_drawing    = old_drawing.into_iter()
                .enumerate()
                .filter(|(idx, _item)| !unused_indexes.contains(idx))
                .map(|(_idx, item)| item)
                .collect();
        }
    }

    ///
    /// Removes any PushState/PopState operation that performs no drawing (ie, just sets up state and throws it away)
    ///
    pub fn remove_unused_state_stack_ops(&mut self) {
        // A stack of push states which have no drawing operations (cleared on any drawing operation)
        let mut push_state_stack    = vec![];

        // The indexes that we should remove
        let mut to_remove           = HashSet::new();

        for (idx, (_tgt_resource, draw)) in self.pending_drawing.iter().enumerate() {
            match draw {
                Draw::PushState => {
                    // This pushes a state onto the stack
                    push_state_stack.push(idx);
                }

                Draw::PopState => {
                    // If there's a 'unaffected' state on top of the push state stack, then clear all the indexes
                    // The stack is cleared when the state is used, so the pop will return None in that case
                    if let Some(initial_idx) = push_state_stack.pop() {
                        for remove_idx in initial_idx..=idx {
                            to_remove.insert(remove_idx);
                        }
                    }
                }

                draw => {
                    // Clear the state stack so nothing will be removed if the operation uses the state that's present there
                    if !draw.is_state_stack_resource() {
                        if push_state_stack.len() > 0 {
                            push_state_stack = vec![];
                        }
                    }
                }
            }
        }

        // Remove anything in the to_remove hashset
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
    /// Removes any commands from the stream that have a source or target of a layer
    ///
    fn remove_all_layer_commands(&mut self) {
        let mut to_remove           = HashSet::new();

        // Queue up any instruction that targets a layer for removal
        for (idx, (target_resource, _draw)) in self.pending_drawing.iter().enumerate() {
            if let DrawResource::Layer(_) = target_resource {
                to_remove.insert(idx);
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
        let mut balance_frames  = false;
        let mut has_stack_ops   = false;

        for draw in drawing {
            // Process the drawing instruction
            match &draw {
                Draw::Layer(layer_id)   => { self.target_resource = DrawResource::Layer(*layer_id); },
                Draw::Sprite(sprite_id) => { self.target_resource = DrawResource::Sprite(*sprite_id); },

                Draw::ClearLayer        |
                Draw::ClearSprite       => { 
                    self.clear_resource(self.target_resource);
                    drawing_cleared = true; 
                    
                    match self.target_resource {
                        DrawResource::Layer(layer_id)   => self.pending_drawing.push((self.target_resource, Draw::Layer(layer_id))),
                        DrawResource::Sprite(sprite_id) => self.pending_drawing.push((self.target_resource, Draw::Sprite(sprite_id))),
                        _                               => unimplemented!()
                    }
                },

                Draw::ClearCanvas(_)    => { 
                    self.pending_drawing.retain(|(tgt, _action)| tgt == &DrawResource::Frame);
                    self.target_resource = DrawResource::Layer(LayerId(0));
                },

                Draw::ClearAllLayers    => {
                    self.remove_all_layer_commands();
                    drawing_cleared = true;
                }

                Draw::PopState          => {
                    has_stack_ops = true;
                }

                _                       => { }
            }

            // Append the drawing instruction to the pending drawing
            match &draw {
                Draw::Restore => {
                    // Add the 'restore' operation in case we can't rewind anything
                    let drawing_target = draw.target_resource(&self.target_resource);
                    self.pending_drawing.push((drawing_target, draw));

                    // Rewind if the action is a 'restore' action
                    self.rewind_to_last_store();
                    drawing_cleared = true;
                },

                Draw::FreeStoredBuffer => {
                    // If we free a stored buffer immediately after a store, then both operations
                    if let Some((_, Draw::Store)) = self.pending_drawing.last() {
                        self.pending_drawing.pop();
                    } else {
                        let drawing_target = draw.target_resource(&self.target_resource);
                        self.pending_drawing.push((drawing_target, draw));
                    }
                }

                Draw::ShowFrame         |
                Draw::ResetFrame        => {
                    balance_frames = true;
                    self.pending_drawing.push((DrawResource::Frame, draw));
                }

                _ => {
                    // Add everything else to the pending drawing
                    let drawing_target = draw.target_resource(&self.target_resource);
                    self.pending_drawing.push((drawing_target, draw));
                }
            }
        }

        // If we've processed a clear instruction, clear out any unused resources from the pending list
        if drawing_cleared {
            self.remove_unused_resources();
        }

        if drawing_cleared && has_stack_ops {
            self.remove_unused_state_stack_ops();
        }

        if balance_frames {
            self.balance_show_frames();
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

    ///
    /// Returns the pending drawing operations in this core
    ///
    pub fn get_pending_drawing<'a>(&'a self) -> impl 'a+Iterator<Item=Draw> {
        self.pending_drawing.iter()
            .map(|(_, drawing)| drawing)
            .cloned()
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
