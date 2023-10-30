use super::canvas_drawing::*;
use super::drawing_state::*;
use super::layer::*;
use super::prepared_layer::*;

use crate::edgeplan::*;
use crate::edges::*;
use crate::pixel::*;
use crate::pixel_programs::*;

use flo_canvas as canvas;
use smallvec::*;

use std::sync::*;

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

            // Use the layer we just created
            self.current_layer = new_layer_handle;
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
    /// Creates or retrieves the 'prepared' version of the current layer, which can be used to render sprites or textures
    ///
    pub fn prepare_layer(&mut self, layer_handle: LayerHandle) -> PreparedLayer {
        if let Some(layer) = self.prepared_layers.get(layer_handle.0) {
            // Use the existing prepared layer
            layer.clone()
        } else if let Some(current_layer) = self.layers.get(layer_handle.0){
            // Prepare the current layer
            let mut current_layer = current_layer.edges.clone();
            current_layer.prepare_to_render();

            // Calculate the overall bounding box of the layer
            let bounds = current_layer.bounding_box();

            // Create the prepared layer
            let prepared_layer = PreparedLayer {
                edges:  Arc::new(current_layer),
                bounds: bounds,
            };

            // Store in the cache (drawing should clear the prepared layer)
            self.prepared_layers.insert(layer_handle.0, prepared_layer.clone());

            prepared_layer
        } else {
            // Layer does not exist
            PreparedLayer {
                edges: Arc::new(EdgePlan::new()),
                bounds: ((0.0, 0.0), (0.0, 0.0))
            }
        }
    }

    ///
    /// Draws the sprite with the specified ID
    ///
    pub fn sprite_draw(&mut self, sprite_id: canvas::SpriteId) {
        use std::iter;

        // Get the layer handle for this sprite
        if let Some(sprite_layer_handle) = self.sprites.get(&(self.current_namespace, sprite_id)) {
            // Prepare the sprite layer for rendering
            let sprite_layer = self.prepare_layer(*sprite_layer_handle);

            if !sprite_layer.edges.is_empty() {
                // Get the z-index of where to render this sprite
                let current_layer   = self.layers.get_mut(self.current_layer.0).unwrap();
                let z_index         = current_layer.z_index;

                current_layer.z_index += 1;

                if let SpriteTransform::ScaleTransform { scale, translate } = self.current_state.sprite_transform {
                    // Transform the coordinates for the current sprite transform
                    // TODO: we also need to apply things like the current canvas transform rotation here
                    let translate = self.current_state.transform.transform_point(translate.0 as _, translate.1 as _);
                    let translate = (translate.0 as f64, translate.1 as f64);

                    // Create the brush data
                    let data    = BasicSpriteData::new(sprite_layer.edges, scale, translate);
                    let data_id = self.program_cache.program_cache.store_program_data(&self.program_cache.basic_sprite, &mut self.program_data_cache, data);

                    // Shape is a transparent rectangle that runs this program
                    let shape_descriptor = ShapeDescriptor {
                        programs:   smallvec![data_id],
                        is_opaque:  false,
                        z_index:    z_index,
                    };
                    let shape_id = ShapeId::new();

                    // Create a rectangle edge for this data (TODO: needs to be transformed)
                    let ((min_x, min_y), (max_x, max_y)) = sprite_layer.bounds;
                    let sprite_edge = RectangleEdge::new(shape_id, min_x..max_x, min_y..max_y);
                    let sprite_edge: Arc<dyn EdgeDescriptor> = Arc::new(sprite_edge);

                    // Store in the current layer
                    current_layer.edges.add_shape(shape_id, shape_descriptor, iter::once(sprite_edge));
                    current_layer.used_data.push(data_id);

                    // This 'unprepares' the current layer as for any other drawing operation
                    self.prepared_layers.remove(self.current_layer.0);
                } else {
                    // Need a way to render edge plans at arbitrary angles to implement this
                    todo!("Only scaling and translations are currently supported for sprites")
                }
            }
        }
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