use super::canvas_renderer::*;
use super::tessellate_build_path::*;

use crate::fill_state::*;
use crate::layer_state::*;
use crate::layer_bounds::*;
use crate::layer_handle::*;
use crate::render_entity::*;
use crate::renderer_layer::*;
use crate::stroke_settings::*;

use flo_canvas as canvas;
use flo_render as render;

use lyon::tessellation::{FillRule};

use std::mem;
use std::sync::*;

impl CanvasRenderer {
    ///
    /// Creates a new layer with the default properties
    ///
    pub (super) fn create_default_layer() -> Layer {
        Layer {
            render_order:               vec![RenderEntity::SetTransform(canvas::Transform2D::identity())],
            state:                      LayerState {
                is_sprite:          false,
                modification_count: 0,
                fill_color:         FillState::Color(render::Rgba8([0, 0, 0, 255])),
                winding_rule:       FillRule::NonZero,
                stroke_settings:    StrokeSettings::new(),
                current_matrix:     canvas::Transform2D::identity(),
                sprite_matrix:      canvas::Transform2D::identity(),
                scale_factor:       0.002,                              // Canvas height of approximately 768 (1.0 will tessellate at far too fine a detail for these coordinate schemes, so we default to 0.002 as a safety net)
                base_scale_factor:  1.0,
                blend_mode:         canvas::BlendMode::SourceOver,
                restore_point:      None
            },
            bounds:                     LayerBounds::default(),
            stored_states:              vec![],
            commit_before_rendering:    false,
            commit_after_rendering:     false,
            blend_mode:                 canvas::BlendMode::SourceOver,
            alpha:                      1.0
        }
    }

    ///
    /// Clears the canvas entirely
    ///
    pub (super) fn tes_clear_canvas(&mut self, background: canvas::Color, path_state: &mut PathState) {
        //todo!("Stop any incoming tessellated data for this layer");
        //todo!("Mark vertex buffers as freed");

        *path_state = PathState::default();
        let core    = Arc::clone(&self.core);

        core.sync(|core| {
            // Release the textures
            let old_textures = mem::take(&mut core.canvas_textures);

            for (_canvas_id, render_id) in old_textures.into_iter() {
                let render_id = (&render_id).into();
                core.used_textures.get_mut(&render_id).map(|usage_count| *usage_count -= 1);
            }

            // Release the existing layers
            let old_layers = mem::take(&mut core.layers);

            for layer_id in old_layers {
                let layer = core.release_layer_handle(layer_id);
                core.free_layer_entities(layer);
            }

            // Release the sprites
            let old_sprites = mem::take(&mut core.sprites);

            for (_sprite_id, layer_id) in old_sprites {
                let layer = core.release_layer_handle(layer_id);
                core.free_layer_entities(layer);
            }

            // Set the background colour for when we start rendering
            core.background_color   = Self::render_color(background);

            // Create a new default layer
            let layer0 = Self::create_default_layer();
            let layer0 = core.allocate_layer_handle(layer0);
            core.layers.push(layer0);

            self.current_layer  = layer0;
            self.current_sprite = None;
        });

        self.active_transform   = canvas::Transform2D::identity();
    }

    ///
    /// Selects a particular layer for drawing
    /// Layer 0 is selected initially. Layers are drawn in order starting from 0.
    /// Layer IDs don't have to be sequential.
    ///
    pub (super) fn tes_layer(&mut self, canvas::LayerId(layer_id): canvas::LayerId) {
        let layer_id    = layer_id as usize;
        let core        = Arc::clone(&self.core);

        // Generate layers 
        core.sync(|core| {
            while core.layers.len() <= layer_id  {
                let new_layer = Self::create_default_layer();
                let new_layer = core.allocate_layer_handle(new_layer);
                core.layers.push(new_layer);
            }

            self.current_layer  = core.layers[layer_id];
            self.current_sprite = None;
        });
    }

    ///
    /// Sets how a particular layer is blended with the underlying layer
    ///
    pub (super) fn tes_layer_blend(&mut self, canvas::LayerId(layer_id): canvas::LayerId, blend_mode: canvas::BlendMode) {
        self.core.sync(move |core| {
            let layer_id = layer_id as usize;

            if layer_id < core.layers.len() {
                // Fetch the layer
                let layer_handle    = core.layers[layer_id];
                let layer           = core.layer(layer_handle);

                // Update the blend mode and set the layer's 'commit' mode
                layer.blend_mode    = blend_mode;
                if blend_mode != canvas::BlendMode::SourceOver {
                    // Need to commit before to stop whatever is under the layer from having the blend mode applied to it, and after to apply the blend mode
                    layer.commit_before_rendering   = true;
                    layer.commit_after_rendering    = true;
                }
            }
        });
    }

    ///
    /// Sets the alpha blend mode for a particular layer
    ///
    pub (super) fn tes_layer_alpha(&mut self, canvas::LayerId(layer_id): canvas::LayerId, layer_alpha: f32) {
        self.core.sync(move |core| {
            let layer_id = layer_id as usize;

            if layer_id < core.layers.len() {
                // Fetch the layer
                let layer_handle    = core.layers[layer_id];
                let layer           = core.layer(layer_handle);

                let layer_alpha     = f32::max(0.0, f32::min(1.0, layer_alpha));

                // Update the alpha value and set the layer's 'commit' mode
                layer.alpha    = layer_alpha as _;
                if layer_alpha < 1.0 {
                    layer.commit_before_rendering   = true;
                    layer.commit_after_rendering    = true;
                }
            }
        });
    }

    ///
    /// Clears the current layer
    ///
    pub (super) fn tes_clear_layer(&mut self, path_state: &mut PathState) {
        *path_state = PathState::default();

        self.core.sync(|core| {
            // Create a new layer
            let mut layer   = Self::create_default_layer();

            // Sprite layers act as if their transform is already set
            let old_layer   = core.layer(self.current_layer);
            if old_layer.state.is_sprite {
                layer.state.is_sprite   = true;
            }

            // Retain the modification count from the old layer
            layer.state.modification_count  = old_layer.state.modification_count + 1;
            layer.state.base_scale_factor   = old_layer.state.base_scale_factor;
            layer.state.scale_factor        = old_layer.state.scale_factor;

            // Swap into the layer list to replace the old one
            mem::swap(core.layer(self.current_layer), &mut layer);

            // Ensure the layer transform is up to date
            core.layer(self.current_layer).update_transform(&self.active_transform);

            // Free the data for the layer that we just replaced
            core.free_layer_entities(layer);
        });
    }

    ///
    /// Clears all of the layers (leaving sprites, textures, etc intact)
    ///
    pub (super) fn tes_clear_all_layers(&mut self, path_state: &mut PathState) {
        *path_state = PathState::default();

        self.core.sync(|core| {
            let handles = core.layers.clone();

            for handle in handles.into_iter() {
                // Sprite layers are left alone
                if core.layer(self.current_layer).state.is_sprite {
                    continue;
                }

                // Create a new layer
                let mut layer   = Self::create_default_layer();

                // Swap into the layer list to replace the old one
                mem::swap(core.layer(handle), &mut layer);

                // Free the data for the current layer
                core.free_layer_entities(layer);
            }
        });
    }

    ///
    /// Swaps two layers (changing their render order)
    ///
    pub (super) fn tes_swap_layers(&mut self, canvas::LayerId(layer1): canvas::LayerId, canvas::LayerId(layer2): canvas::LayerId) {
        if layer1 != layer2 {
            self.core.sync(move |core| {
                // Create layers if they don't already exist so we can swap with arbitrary layers
                let max_layer_id = u64::max(layer1, layer2) as usize;
                while core.layers.len() <= max_layer_id  {
                    let new_layer = Self::create_default_layer();
                    let new_layer = core.allocate_layer_handle(new_layer);
                    core.layers.push(new_layer);
                }

                // Swap the two layers in the core
                let LayerHandle(handle1) = core.layers[layer1 as usize];
                let LayerHandle(handle2) = core.layers[layer2 as usize];

                if handle1 != handle2 {
                    core.layer_definitions.swap(handle1 as usize, handle2 as usize);
                }
            });
        }
    }
}