use crate::draw::*;

use flo_scene::*;

use std::sync::*;

///
/// A request to a 2D drawing target
///
#[derive(Debug, Clone)]
pub enum DrawingRequest {
    /// Perform the specified drawing actions
    Draw(Arc<Vec<Draw>>),
}

impl SceneMessage for DrawingRequest {  }
