use super::canvas_renderer::*;

use flo_canvas as canvas;

impl CanvasRenderer {
    ///
    /// Clears the currently selected sprite
    ///
    #[inline]
    pub (super) fn tes_namespace(&mut self, namespace: canvas::NamespaceId) {
        // The current namespace is used to identify different groupds of resources
        self.current_namespace = namespace.local_id();
    }
}
