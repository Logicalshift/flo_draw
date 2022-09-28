use super::canvas_renderer::*;
use super::tessellate_build_path::*;

use crate::texture_filter_request::*;

use crate::render_entity::*;

use flo_canvas as canvas;

use std::sync::*;

impl CanvasRenderer {
    ///
    /// Clears the currently selected sprite
    ///
    #[inline]
    pub (super) fn tes_clear_sprite(&mut self, path_state: &mut PathState) {
        // Sprites are just layers that we don't render in the standard stack
        self.tes_clear_layer(path_state);
    }

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
                self.current_layer  = *sprite_handle;
                self.current_sprite = Some(sprite_id);
            } else {
                // Create a new sprite layer
                let mut sprite_layer            = Self::create_default_layer();
                sprite_layer.state.is_sprite    = true;

                // Associate it with the sprite ID
                let sprite_layer                = core.allocate_layer_handle(sprite_layer);
                core.sprites.insert(sprite_id, sprite_layer);

                // Choose the layer as the current sprite layer
                self.current_layer  = sprite_layer;
                self.current_sprite = Some(sprite_id);
            }

            // Set the sprite matrix to be 'unchanged' from the active transform
            let layer                   = core.layer(self.current_layer);
            layer.update_transform(&self.active_transform);

            // Set the scale factor in the sprite layer
            layer.state.base_scale_factor   = previous_layer_scale_factor;
            layer.state.scale_factor        = previous_layer_scale_factor;
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

            // Update the transformation matrix for the layer
            layer.update_transform(&self.active_transform);

            // Render the sprite
            layer.render_order.push(RenderEntity::RenderSprite(sprite_id, sprite_matrix));
            layer.state.modification_count += 1;
        })
    }

    ///
    /// Renders a sprite with a set of transformations and filters
    ///
    pub (super) fn tes_draw_sprite_with_filters(&mut self, sprite_id: canvas::SpriteId, filters: Vec<canvas::TextureFilter>) { 
        self.core.sync(|core| {
            let layer           = core.layer(self.current_layer);
            let sprite_matrix   = layer.state.sprite_matrix;

            // Update the transformation matrix for the layer
            layer.update_transform(&self.active_transform);

            // Turn the TextureFilters into filter requests
            let filters = filters.into_iter().filter_map(|filter| {
                use canvas::TextureFilter::*;

                match filter {
                    GaussianBlur(radius)                => Some(TextureFilterRequest::CanvasBlur(radius, self.active_transform)),
                    AlphaBlend(alpha)                   => Some(TextureFilterRequest::AlphaBlend(alpha)),
                    Mask(texture)                       => Some(TextureFilterRequest::Mask(core.texture_for_rendering(texture)?)),
                    DisplacementMap(texture, xr, yr)    => Some(TextureFilterRequest::DisplacementMap(core.texture_for_rendering(texture)?, xr, yr, Some(self.active_transform))),
                }
            }).collect::<Vec<_>>();

            // Increase the usage count of any referenced textures
            for texture_id in filters.iter().flat_map(|filter| filter.used_textures()) {
                core.add_texture_usage(texture_id);
            }

            // Render the sprite
            let layer = core.layer(self.current_layer);
            layer.render_order.push(RenderEntity::RenderSpriteWithFilters(sprite_id, sprite_matrix, filters));
            layer.state.modification_count += 1;
        })
    }

    ///
    /// Moves a definition from a different sprite ID to this one
    ///
    pub (super) fn tes_move_sprite_from(&mut self, move_from_sprite_id: canvas::SpriteId, path_state: &mut PathState) {
        // Fetch the current sprite, or do nothing if a sprite is not selected
        let current_sprite_id = if let Some(sprite_id) = self.current_sprite { sprite_id } else { return; };

        // Moving a sprite to itself is also a no-op
        if current_sprite_id == move_from_sprite_id {
            return;
        }

        // Clear the current layer
        self.tes_clear_layer(path_state);

        self.core.sync(|core| {
            // Remove the definition from the existing sprite
            if let Some(sprite_layer_handle) = core.sprites.remove(&move_from_sprite_id) {
                // Set the current sprite to use the layer we just removed
                core.sprites.insert(current_sprite_id, sprite_layer_handle);
            }
        })
    }

    ///
    /// Copies a definition from a different sprite ID
    ///
    pub (super) fn tes_copy_sprite_from(&mut self, sprite_id: canvas::SpriteId, path_state: &mut PathState) {
        todo!()
    }
}
