use super::canvas_drawing::*;

use crate::pixel::*;
use crate::edgeplan::*;

use flo_canvas as canvas;

use std::sync::*;

///
/// A layer handle is a reference to a layer within a drawing
///
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct LayerHandle(pub usize);

///
/// Data associated with a drawing layer
///
pub struct Layer {
    /// The transparency of this layer
    pub (super) alpha: f64,

    /// The transform that was applied to this layer last time it was selected or deselected
    pub (super) last_transform: canvas::Transform2D,

    /// The blending function used for this layer
    pub (super) blend_mode: AlphaOperation,

    /// The edges that make up this layer
    pub (super) edges: EdgePlan<Arc<dyn EdgeDescriptor>>,

    /// The pixel program data referenced by this layer
    pub (super) used_data: Vec<PixelProgramDataId>,

    /// The edges that were stored with the 'Store' command
    pub (super) stored_edges: Vec<Arc<dyn EdgeDescriptor>>,

    /// The program data in use when the edges were stored
    pub (super) stored_data: Vec<PixelProgramDataId>,

    /// The z-index for the next shape we add to the edge plan
    pub (super) z_index: i64,

    /// The total number of times this layer has been changed
    pub (super) edit_count: usize,

    /// The transform to apply to this layer when rendering it
    pub (super) layer_transform: canvas::Transform2D,

    /// If the layer is a rotation, these are the transformed edges we use for the 
    pub (super) transformed_edges: Option<EdgePlan<Arc<dyn EdgeDescriptor>>>,
}

impl Default for Layer {
    fn default() -> Self {
        Layer { 
            alpha:              1.0,
            last_transform:     canvas::Transform2D::identity(),
            blend_mode:         AlphaOperation::SourceOver,
            edges:              EdgePlan::new(),
            stored_edges:       vec![],
            used_data:          vec![],
            stored_data:        vec![],
            z_index:            0,
            edit_count:         0,
            layer_transform:    canvas::Transform2D::identity(),
            transformed_edges:  None,
        }
    }
}

impl Layer {
    ///
    /// Clears this layer
    ///
    /// This leaves the program data and stored edges intact, so this needs to be released separately
    ///
    pub fn clear(&mut self) {
        self.alpha      = 1.0;
        self.blend_mode = AlphaOperation::SourceOver;
        self.edges      = EdgePlan::new();
        self.z_index    = 0;
    }
}

impl<TPixel, const N: usize> CanvasDrawing<TPixel, N> 
where
    TPixel: 'static + Send + Sync + Pixel<N>,
{
    ///
    /// Creates a new blank layer and returns the layer ID that it will have
    ///
    pub (crate) fn create_new_layer(&mut self, namespace: usize, layer_id: canvas::LayerId) {
        if self.handle_for_layer.contains_key(&(namespace, layer_id)) {
            // Layer is already created
            return;
        }

        // Create a layer and a handle
        let new_layer           = Layer::default();
        let new_layer_handle    = self.next_layer_handle;

        self.layers.insert(new_layer_handle.0, new_layer);

        // Advance the next layer handle
        self.next_layer_handle.0 += 1;

        // The new layer has to appear after whichever layer is 'before' it (has a lower layer ID and the same namespace)
        // If there's no other layers in the namespace, then the layer is ordered last
        let mut previous_layer_handle   = None;
        let mut closest_layer_id        = 0;
        for ((test_namespace, test_layer_id), layer_handle) in self.handle_for_layer.iter() {
            if *test_namespace == namespace && test_layer_id.0 <= layer_id.0 && test_layer_id.0 >= closest_layer_id {
                previous_layer_handle   = Some(*layer_handle);
                closest_layer_id        = test_layer_id.0;
            }
        }

        // Add to the handle map
        self.handle_for_layer.insert((namespace, layer_id), new_layer_handle);

        // Add to the layer order
        if let Some(previous_layer_handle) = previous_layer_handle {
            // Figure out where to add in the ordered layer list
            let previous_layer_idx = self.ordered_layers.iter().position(|handle| *handle == previous_layer_handle);
            debug_assert!(previous_layer_idx.is_some());

            if let Some(previous_layer_idx) = previous_layer_idx {
                self.ordered_layers.insert(previous_layer_idx+1, new_layer_handle);
            } else {
                self.ordered_layers.push(new_layer_handle);
            }
        } else {
            // If there's no previous layer handle, then create a new namespace
            if layer_id != canvas::LayerId(0) {
                // Ensure that layer 0 is created for this layer so that other layers order properly
                self.create_new_layer(namespace, canvas::LayerId(0));
            }

            // This layer goes last (as at most layer 0 exists for the new namespace now)
            self.ordered_layers.push(new_layer_handle)
        }
    }

    ///
    /// Ensures that a particular layer exists
    ///
    #[inline]
    pub (crate) fn ensure_layer(&mut self, layer_id: canvas::LayerId) {
        // Add layers until we get to the current layer ID
        if !self.handle_for_layer.contains_key(&(self.current_namespace, layer_id)) {
            self.create_new_layer(self.current_namespace, layer_id)
        }
    }

    ///
    /// Retrieves the layer with the specified ID (if it has been created)
    ///
    #[inline]
    pub (crate) fn layer_with_id(&mut self, namespace_id: usize, layer_id: canvas::LayerId) -> Option<&mut Layer> {
        if let Some(handle) = self.handle_for_layer.get(&(namespace_id, layer_id)) {
            let handle = *handle;
            self.layers.get_mut(handle.0 as usize)
        } else {
            None
        }
    }

    ///
    /// Retrieves the layer with the specified ID(if it has been created)
    ///
    #[inline]
    pub (crate) fn layer_with_id_readonly(&self, namespace_id: usize, layer_id: canvas::LayerId) -> Option<&Layer> {
        if let Some(handle) = self.handle_for_layer.get(&(namespace_id, layer_id)) {
            let handle = *handle;
            self.layers.get(handle.0 as usize)
        } else {
            None
        }
    }

    ///
    /// Retrieves the layer with a particular handle
    ///
    #[inline]
    pub (crate) fn layer(&mut self, layer_handle: LayerHandle) -> Option<&mut Layer> {
        self.layers.get_mut(layer_handle.0)
    }

    ///
    /// Selects or creates a layer with the given ID
    ///
    #[inline]
    pub (crate) fn select_layer(&mut self, layer_id: canvas::LayerId) {
        let transform = self.current_state.transform;

        // Add layers until we get to the current layer ID
        self.ensure_layer(layer_id);

        // Update the transform of the layer we're leaving
        if let Some(layer) = self.layer(self.current_layer) { layer.last_transform = transform; }

        // Pick this layer
        self.current_layer = *self.handle_for_layer.get(&(self.current_namespace, layer_id)).unwrap();

        // Update the transform of the layer we're entering
        if let Some(layer) = self.layer(self.current_layer) { layer.last_transform = transform; }
    }

    ///
    /// Clears a layer
    ///
    #[inline]
    pub (crate) fn clear_layer(&mut self, handle: LayerHandle) {
        if let Some(layer) = self.layers.get_mut(handle.0) {
            // Clear the layer
            layer.clear();

            // Release the layer's data
            // There is also data in stored_data, that's in use by the stored edges: that's not freed here in case we want to restore those edges
            for data_id in layer.used_data.drain(..) {
                self.program_data_cache.release_program_data(data_id);
            }
        }
    }

    ///
    /// Creates a clone of a layer (retaining any resources that it's using)
    ///
    pub (crate) fn clone_layer(&mut self, handle: LayerHandle) -> Layer {
        if let Some(layer) = self.layers.get(handle.0) {
            // Retain the program data for this layer
            for data_id in layer.used_data.iter() {
                self.program_data_cache.retain_program_data(*data_id);
            }

            for data_id in layer.stored_data.iter() {
                self.program_data_cache.retain_program_data(*data_id);
            }

            // Create a copy of the layer
            Layer {
                alpha:              layer.alpha,
                last_transform:     layer.last_transform,
                blend_mode:         layer.blend_mode,
                edges:              layer.edges.clone(),
                used_data:          layer.used_data.clone(),
                stored_edges:       layer.stored_edges.clone(),
                stored_data:        layer.stored_data.clone(),
                z_index:            layer.z_index,
                edit_count:         layer.edit_count,
                layer_transform:    layer.layer_transform,
                transformed_edges:  None,
            }
        } else {
            // Just use an empty default layer if this layer isn't created yet
            Layer::default()
        }
    }

    ///
    /// Sets the blend mode of a layer
    ///
    #[inline]
    pub (crate) fn layer_blend(&mut self, layer_id: canvas::LayerId, blend: canvas::BlendMode) {
        use canvas::BlendMode::*;

        self.ensure_layer(layer_id);

        let operation = match blend {
            SourceOver          => { AlphaOperation::SourceOver },
            SourceIn            => { AlphaOperation::SourceIn },
            SourceOut           => { AlphaOperation::SourceHeldOut },
            DestinationOver     => { AlphaOperation::DestOver },
            DestinationIn       => { AlphaOperation::DestIn },
            DestinationOut      => { AlphaOperation::DestHeldOut },
            SourceAtop          => { AlphaOperation::SourceAtop },
            DestinationAtop     => { AlphaOperation::DestAtop },

            Multiply            => { todo!() },
            Screen              => { todo!() },
            Darken              => { todo!() },
            Lighten             => { todo!() },
        };

        if let Some(layer) = self.layer_with_id(self.current_namespace, layer_id) {
            layer.blend_mode = operation;
        }
    }

    ///
    /// Sets the alpha factor of a layer
    ///
    #[inline]
    pub (crate) fn layer_alpha(&mut self, layer_id: canvas::LayerId, alpha: f64) {
        self.ensure_layer(layer_id);

        if let Some(layer) = self.layer_with_id(self.current_namespace, layer_id) {
            layer.alpha = alpha;
        }
    }

    ///
    /// Clears all of the layers in the current drawing
    ///
    #[inline]
    pub (crate) fn clear_all_layers(&mut self) {
        let ordered_layers      = &mut self.ordered_layers;
        let layers              = &mut self.layers;
        let program_data_cache  = &mut self.program_data_cache;

        ordered_layers.iter_mut()
            .for_each(|layer_handle| {
                let layer = layers.get_mut(layer_handle.0);
                if let Some(layer) = layer {
                    layer.clear();

                    // Release the layer's data
                    for data_id in layer.used_data.drain(..) {
                        program_data_cache.release_program_data(data_id);
                    }
                }
            });
    }

    ///
    /// Swaps two layers over
    ///
    #[inline]
    pub (crate) fn swap_layers(&mut self, layer_1: canvas::LayerId, layer_2: canvas::LayerId) {
        // Layers must exist
        self.ensure_layer(layer_1);
        self.ensure_layer(layer_2);

        // Swap the two indexes in the ordered layer list
        self.ordered_layers.swap(layer_1.0 as usize, layer_2.0 as usize);
    }

    ///
    /// Sets the current layer to draw before the specified layer
    ///
    pub (crate) fn place_layer_before(&mut self, namespace_id: canvas::NamespaceId, layer_id: canvas::LayerId) {
        // Get the handles of the layer that we're moving and the one we're moving before
        let moving_layer                = self.current_layer;
        let Some(before_layer_handle)   = self.handle_for_layer.get(&(namespace_id.local_id(), layer_id)) else { return; };
        let before_layer_handle         = *before_layer_handle;

        // Can't move a layer before itself
        if before_layer_handle == moving_layer {
            return;
        }

        // Remove the layer that's being moved from the ordered layer list
        let initial_len = self.ordered_layers.len();
        self.ordered_layers.retain(|layer| layer != &moving_layer);

        // If the layer we're moving is not in the list, then there's nothing to do
        if initial_len == self.ordered_layers.len() {
            return;
        }

        // Find the 'before' layer in the ordered layer list and add the moving layer before it
        // We're assuming everything in handle_for_layer is also in ordered_layers (which should be true)
        let before_idx = self.ordered_layers.iter().position(|layer| layer == &before_layer_handle);

        if let Some(before_idx) = before_idx {
            self.ordered_layers.insert(before_idx, moving_layer);
        } else {
            panic!("Layer is missing from the ordered layers list");
        }
    }

    ///
    /// Sets the transform to apply to the layer
    ///
    pub (crate) fn set_layer_transform(&mut self, layer_transform: canvas::Transform2D) {
        let current_transform = self.current_state.transform;

        if let Some(layer) = self.layer(self.current_layer) {
            if let Some(inverse_transform) = current_transform.invert() {
                layer.layer_transform = current_transform * layer_transform * inverse_transform;
            }
        }
    }

    ///
    /// Stores the edges in the current layer in a cache
    ///
    #[inline]
    pub (crate) fn store_layer_edges(&mut self) {
        // Make sure the stored edges are empty
        self.free_stored_edges();

        // Borrow the data we need
        let program_data_cache  = &mut self.program_data_cache;
        let layers              = &mut self.layers;
        let layer               = &mut layers.get_mut(self.current_layer.0).unwrap();

        // Store the current set of edges 
        layer.stored_edges.extend(layer.edges.all_edges().cloned());

        // Add extra references to the program data for the stored edges
        layer.stored_data.extend(layer.used_data.iter()
            .map(|data_id| {
                program_data_cache.retain_program_data(*data_id);
                *data_id
            }));
    }

    ///
    /// Restores the edges in the current layer from the cache
    ///
    #[inline]
    pub (crate) fn restore_layer_edges(&mut self) {
        // Borrow the data we need
        let program_data_cache  = &mut self.program_data_cache;
        let layers              = &mut self.layers;
        let layer               = &mut layers.get_mut(self.current_layer.0).unwrap();
        let edges               = &mut layer.edges;
        let stored_edges        = &layer.stored_edges;

        // Clear the existing set of edges and replace with the stored edges    
        edges.clear_edges();
        stored_edges.iter()
            .cloned()
            .for_each(|edge| edges.add_edge(edge));

        // Clear the existing program data and replace with the stored program data
        layer.used_data.drain(..).for_each(|data_id| program_data_cache.release_program_data(data_id));
        layer.used_data.extend(layer.stored_data.iter()
            .map(|data_id| {
                program_data_cache.retain_program_data(*data_id);
                *data_id
            }));
        layer.edit_count += 1;

        self.prepared_layers.remove(self.current_layer.0);
    }

    ///
    /// Removes the stored edges for the current layer
    ///
    #[inline]
    pub (crate) fn free_stored_edges(&mut self) {
        // Borrow the things we need
        let program_data_cache  = &mut self.program_data_cache;
        let layers              = &mut self.layers;
        let layer               = &mut layers.get_mut(self.current_layer.0).unwrap();
        let stored_edges        = &mut layer.stored_edges;
        let stored_data         = &mut layer.stored_data;
    
        // Free the edges
        stored_edges.clear();

        // Release the data associated with each edge
        stored_data.drain(..).for_each(|data_id| program_data_cache.release_program_data(data_id));
    }
}
