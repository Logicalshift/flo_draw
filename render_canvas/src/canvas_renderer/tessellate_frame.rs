use super::canvas_renderer::*;

impl CanvasRenderer {
    /// Suspends rendering to the display until the next 'ShowFrame'
    ///
    /// The renderer may perform tessellation or rendering in the background after 'StartFrame' but won't
    /// commit anything to the visible frame buffer until 'ShowFrame' is hit. If 'StartFrame' is nested,
    /// then the frame won't be displayed until 'ShowFrame' has been requested at least that many times.
    ///
    /// The frame state persists across a 'ClearCanvas'
    #[inline]
    pub (super) fn tes_start_frame(&self) {
        self.core.desync(|core| {
            core.frame_starts += 1;
        });
    }

    /// Displays any requested queued after 'StartFrame'
    #[inline]
    pub (super) fn tes_show_frame(&self) {
        self.core.desync(|core| {
            if core.frame_starts > 0 { 
                core.frame_starts -= 1;
            }
        });
    }

    /// Resets the frame count back to 0 (for when regenerating the state of a canvas)
    #[inline]
    pub (super) fn tes_reset_frame(&self) {
        self.core.desync(|core| {
            core.frame_starts = 0;
        });
    }
}
