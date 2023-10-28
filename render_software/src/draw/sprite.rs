use super::canvas_drawing::*;
use super::layer::*;

use flo_canvas as canvas;

use crate::pixel::*;

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
}
