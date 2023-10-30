use super::canvas_drawing::*;
use super::drawing_state::*;
use super::layer::*;

use flo_canvas as canvas;

use crate::pixel::*;

impl SpriteTransform {
    ///
    /// Returns this transform as a transformation matrix indicating how the points should be transformed
    ///
    #[inline]
    pub fn matrix(&self) -> canvas::Transform2D {
        match self {
            SpriteTransform::ScaleTransform { scale, translate } =>
                canvas::Transform2D::scale(scale.0 as _, scale.1 as _) * canvas::Transform2D::translate(translate.0 as _, translate.0 as _),

            SpriteTransform::Matrix(matrix) => *matrix
        }
    }
}

impl<TPixel, const N: usize> CanvasDrawing<TPixel, N>
where
    TPixel: 'static + Send + Sync + Pixel<N>,
{
    ///
    /// Selects a sprite for rendering
    ///
    #[inline]
    pub fn sprite(&mut self, sprite_id: canvas::SpriteId) {
        let namespace_id = self.current_namespace;

        if let Some(sprite_layer) = self.sprites.get(&(namespace_id, sprite_id)) {
            // Use the existing sprite layer
            self.current_layer = *sprite_layer;
        } else {
            // Create a new sprite layer (sprites are normal layers that aren't rendered until requested)
            let new_layer           = Layer::default();
            let new_layer_handle    = self.next_layer_handle;

            // Advance the next layer handle
            self.next_layer_handle.0 += 1;

            // Add the new layer to the list
            self.layers.insert(new_layer_handle.0, new_layer);
        }
    }

    ///
    /// Moves the content of the specified sprite to the current layer
    ///
    pub fn sprite_move_from(&mut self, sprite_id: canvas::SpriteId) {
        let namespace_id = self.current_namespace;

        // Clear the current layer to release any resources it's using
        self.clear_layer(self.current_layer);

        if let Some(sprite_layer_handle) = self.sprites.get(&(namespace_id, sprite_id)) {
            // Copy the sprite layer
            let sprite_layer_handle = *sprite_layer_handle;
            let layer_copy          = self.clone_layer(sprite_layer_handle);

            // Replace the current layer with the sprite layer
            self.layers.insert(self.current_layer.0, layer_copy);
        }
    }

    ///
    /// Draws the sprite with the specified ID
    ///
    pub fn sprite_draw(&mut self, sprite_id: canvas::SpriteId) {
        // TODO
        // Get the size of the sprite
        // Create the brush data
        // Create a rectangle edge and use the sprite brush
    }
}

impl DrawingState {
    ///
    /// Applies a canvas sprite transform to the current drawing state
    ///
    pub fn sprite_transform(&mut self, transform: canvas::SpriteTransform) {
        use canvas::SpriteTransform::*;

        let sprite_transform = &mut self.sprite_transform;

        match (transform, sprite_transform) {
            (Identity, transform)                                                   => *transform = SpriteTransform::ScaleTransform { scale: (1.0, 1.0), translate: (0.0, 0.0) },

            (Translate(x, y), SpriteTransform::ScaleTransform { translate, scale }) => { translate.0 -= x as f64 * scale.0; translate.1 -= y as f64 * scale.0; }
            (Scale(x, y), SpriteTransform::ScaleTransform { scale, .. })            => { scale.0 /= x as f64; scale.1 /= y as f64; }

            (Rotate(theta), sprite_transform)                                       => { *sprite_transform = SpriteTransform::Matrix(sprite_transform.matrix() * canvas::Transform2D::rotate_degrees(theta)); }
            (Transform2D(matrix), sprite_transform)                                 => { *sprite_transform = SpriteTransform::Matrix(sprite_transform.matrix() * matrix); }
        
            (Translate(x, y), SpriteTransform::Matrix(t))                           => { *t = *t * canvas::Transform2D::translate(x, y); }
            (Scale(x, y), SpriteTransform::Matrix(t))                               => { *t = *t * canvas::Transform2D::scale(x, y); }
        }
    }
}