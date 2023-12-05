use super::drawing_state::*;

use flo_canvas as canvas;

impl DrawingState {
    ///
    /// Resets the transform set in this state
    ///
    #[inline]
    pub (crate) fn identity_transform(&mut self) {
        self.transform = canvas::Transform2D::scale(1.0, -1.0);
    }

    ///
    /// Multiplies the transform by another one
    ///
    #[inline]
    pub (crate) fn multiply_transform(&mut self, transform: canvas::Transform2D) {
        self.transform = self.transform * transform;
    }

    ///
    /// Sets the transform so that the y ranges goes from -(height/2) to (height/2)
    ///
    #[inline]
    pub (crate) fn canvas_height(&mut self, height: f32) {
        // Default transform gives -1, 1 as the height of the window, so the overall height is 2
        let window_height   = 2.0;

        // Work out the scale to use for this widget
        let height          = f32::max(0.0000001, height);
        let scale           = window_height / height;
        let scale           = canvas::Transform2D::scale(scale, -scale);

        // (0, 0) is already the center of the window
        let transform       = scale;

        // Set as the active transform
        self.transform      = transform;
    }

    ///
    /// Sets the transform so that a particular region is centered in the viewport
    ///
    pub (super) fn center_region(&mut self, (x1, y1): (f32, f32), (x2, y2): (f32, f32)) {
        // Get the center point in viewport coordinates
        let center_x                = 0.0;
        let center_y                = 0.0;

        // Find the current center point
        let current_transform       = self.transform.clone();
        let inverse_transform       = current_transform.invert().unwrap();

        let (center_x, center_y)    = inverse_transform.transform_point(center_x, center_y);

        // Translate the center point onto the center of the region
        let (new_x, new_y)          = ((x1+x2)/2.0, (y1+y2)/2.0);
        let translation             = canvas::Transform2D::translate(-(new_x - center_x), -(new_y - center_y));

        self.transform              = self.transform * translation;
    }
}
