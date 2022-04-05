use super::canvas_renderer::*;

use crate::render_entity::*;

use flo_canvas as canvas;

use std::sync::*;

impl CanvasRenderer {
    ///
    /// Selects a particular sprite for drawing
    ///
    pub (super) fn tes_sprite(&mut self, sprite_id: canvas::SpriteId) { 
        let core = Arc::clone(&self.core);

        core.sync(|core| {
            // Update the transform in the current layer, so the scale factor is correct
            core.layer(self.current_layer).update_transform(&self.active_transform);

            // We transfer the scale factor from the current layer to the sprite layer (this is because sprite layers
            // otherwise don't get transformation matrices, so we tessellate them as they would appear on the current 
            // layer). When switching between sprite layers the scale factor also gets inherited from the last non-sprite
            // layer this way.
            let previous_layer_scale_factor = core.layer(self.current_layer).state.scale_factor;

            if let Some(sprite_handle) = core.sprites.get(&sprite_id) {
                // Use the existing sprite layer if one exists
                self.current_layer = *sprite_handle;
            } else {
                // Create a new sprite layer
                let mut sprite_layer            = Self::create_default_layer();
                sprite_layer.state.is_sprite    = true;

                // Associate it with the sprite ID
                let sprite_layer                = core.allocate_layer_handle(sprite_layer);
                core.sprites.insert(sprite_id, sprite_layer);

                // Choose the layer as the current sprite layer
                self.current_layer              = sprite_layer;
            }

            // Set the sprite matrix to be 'unchanged' from the active transform
            let layer                   = core.layer(self.current_layer);
            layer.update_transform(&self.active_transform);

            // Set the scale factor in the sprite layer
            layer.state.scale_factor = previous_layer_scale_factor;
        })
    }

    ///
    /// Adds a sprite transform to the current list of transformations to apply
    ///
    pub (super) fn tes_sprite_transform(&mut self, transform: canvas::SpriteTransform) {
        self. core.sync(|core| {
            core.layer(self.current_layer).state.apply_sprite_transform(transform)
        })
    }

    ///
    /// Renders a sprite with a set of transformations
    ///
    pub (super) fn tes_draw_sprite(&mut self, sprite_id: canvas::SpriteId) { 
        self.core.sync(|core| {
            let layer           = core.layer(self.current_layer);
            let sprite_matrix   = layer.state.sprite_matrix;

            // Update the transformation matrix
            layer.update_transform(&self.active_transform);

            // Render the sprite
            layer.render_order.push(RenderEntity::RenderSprite(sprite_id, sprite_matrix));
            layer.state.modification_count += 1;
        })
    }
}