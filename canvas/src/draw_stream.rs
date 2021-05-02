use crate::draw::*;
use crate::draw_resource::*;

use ::desync::*;
use futures::prelude::*;
use smallvec::*;

use std::sync::*;
use std::collections::{VecDeque, HashSet, HashMap};

///
/// The draw stream core contains the shared data structures for a stream of drawing instructions
///
pub (crate) struct DrawStreamCore {
    /// The pending drawing instructions, and the resource that it affects
    pending_drawing: VecDeque<(DrawResource, Draw)>,

    /// The resource that the stream is currently drawing to
    target_resource: DrawResource,
}

///
/// A draw stream relays `Draw` instructions from a source such as a `Canvas` or a `DrawContext` as a stream
///
pub struct DrawStream {
    /// The core of this draw stream
    core: Arc<Desync<DrawStreamCore>>
}

impl DrawStreamCore {
    ///
    /// Creates a new stream core
    ///
    pub fn new() -> DrawStreamCore {
        // No drawing instructions, and drawing to layer 0 by default
        DrawStreamCore {
            pending_drawing: VecDeque::new(),
            target_resource: DrawResource::Layer(LayerId(0))
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
                    self.pending_drawing = VecDeque::new();
                    self.target_resource = DrawResource::Layer(LayerId(0));
                },

                _                       => { }
            }

            // Add to the pending drawing
            let drawing_target = draw.target_resource(&self.target_resource);
            self.pending_drawing.push_back((drawing_target, draw));
        }

        // If we've processed a clear instruction, clear out any unused resources from the pending list
        if drawing_cleared {
            self.remove_unused_resources();
        }
    }
}
