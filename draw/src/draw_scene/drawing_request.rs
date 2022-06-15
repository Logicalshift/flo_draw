use flo_canvas::*;

use std::sync::*;

///
/// A request to a 2D drawing target
///
pub enum DrawRequest {
    /// Perform the specified drawing actions
    Draw(Arc<Vec<Draw>>),
}
