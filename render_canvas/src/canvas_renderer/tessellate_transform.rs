use super::canvas_renderer::*;

use crate::render_entity::*;

use flo_canvas as canvas;

impl CanvasRenderer {
    /// Reset the transformation to the identity transformation
    pub (super) fn tes_identity_transform(&mut self) {
        self.active_transform = canvas::Transform2D::identity();
    }

    /// Sets a transformation such that:
    /// (0,0) is the center point of the canvas
    /// (0,height/2) is the top of the canvas
    /// Pixels are square
    pub (super) fn tes_canvas_height(&mut self, height: f32) {
        // Window height is set at 2.0 by the viewport transform
        let window_height       = 2.0;

        // Work out the scale to use for this widget
        let height              = f32::max(1.0, height);
        let scale               = window_height / height;
        let scale               = canvas::Transform2D::scale(scale, scale);

        // (0, 0) is already the center of the window
        let transform           = scale;

        // Set as the active transform
        self.active_transform   = transform;
    }

    /// Moves a particular region to the center of the canvas (coordinates are minx, miny, maxx, maxy)
    pub (super) fn tes_center_region(&mut self, (x1, y1): (f32, f32), (x2, y2): (f32, f32)) {
        // Get the center point in viewport coordinates
        let center_x                = 0.0;
        let center_y                = 0.0;

        // Find the current center point
        let current_transform       = self.active_transform.clone();
        let inverse_transform       = current_transform.invert().unwrap();

        let (center_x, center_y)    = inverse_transform.transform_point(center_x, center_y);

        // Translate the center point onto the center of the region
        let (new_x, new_y)          = ((x1+x2)/2.0, (y1+y2)/2.0);
        let translation             = canvas::Transform2D::translate(-(new_x - center_x), -(new_y - center_y));

        self.active_transform       = self.active_transform * translation;
    }

    /// Multiply a 2D transform into the canvas
    pub (super) fn tes_multiply_transform(&mut self, transform: canvas::Transform2D) {
        // Update the active transform: it's applied next time we draw something
        self.active_transform = self.active_transform * transform;

        if self.current_layer_is_sprite {
            // For sprite layers: apply the transform immediately
            // (Sprites do this because they have a default transform of 1.0 and aren't affected by center_region or canvas_height)
            self.core.sync(|core| {
                let layer                   = core.layer(self.current_layer);

                let new_transform           = layer.state.current_matrix * transform;
                layer.state.current_matrix  = new_transform;

                layer.render_order.push(RenderEntity::SetTransform(new_transform));
            });
        }
    }
}
