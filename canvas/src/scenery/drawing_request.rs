use crate::draw::*;

use std::sync::*;

///
/// A request to a 2D drawing target
///
#[derive(Debug, Clone)]
pub enum DrawingRequest {
    /// Perform the specified drawing actions
    Draw(Arc<Vec<Draw>>),
}
