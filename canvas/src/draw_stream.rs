use crate::draw::*;
use crate::draw_resource::*;

use ::desync::*;
use futures::prelude::*;
use smallvec::*;

use std::sync::*;
use std::collections::{VecDeque, HashMap};

///
/// The draw stream core contains the shared data structures for a stream of drawing instructions
///
pub (crate) struct DrawStreamCore {
    /// If there's a next frame that's partially built, the list of drawing instructions pending for that frame
    pending_frame: Vec<Draw>,

    /// The pending drawing instructions, and the resource that it affects
    pending_drawing: VecDeque<(DrawResource, Draw)>
}

///
/// A draw stream relays `Draw` instructions from a source such as a `Canvas` or a `DrawContext` as a stream
///
pub struct DrawStream {
    /// The core of this draw stream
    core: Arc<Desync<DrawStreamCore>>
}
