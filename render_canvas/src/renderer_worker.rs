use super::renderer_core::*;

use ::desync::*;

use std::sync::*;

///
/// State of a canvas worker
///
pub struct CanvasWorker {
    /// The core, where this worker will write its results
    core: Arc<Desync<RenderCore>>
}

impl CanvasWorker {
    ///
    /// Creates a new canvas worker
    ///
    pub fn new(core: &Arc<Desync<RenderCore>>) -> CanvasWorker {
        CanvasWorker {
            core: Arc::clone(core)
        }
    }
}
