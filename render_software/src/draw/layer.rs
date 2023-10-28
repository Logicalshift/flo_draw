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
}

impl Default for Layer {
    fn default() -> Self {
        Layer { 
            alpha:          1.0,
            blend_mode:     AlphaOperation::SourceOver,
            edges:          EdgePlan::new(),
            stored_edges:   vec![],
            used_data:      vec![],
            stored_data:    vec![],
            z_index:        0,
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
    #[inline]
    pub (crate) fn create_new_layer(&mut self) -> canvas::LayerId {
        // Create a layer and a handle
        let new_layer           = Layer::default();
        let new_layer_id        = canvas::LayerId(self.ordered_layers.len() as u64);
        let new_layer_handle    = self.next_layer_handle;

        // Advance the next layer handle
        self.next_layer_handle.0 += 1;

        // Store the new layer
        self.layers.insert(new_layer_handle.0, new_layer);
        self.ordered_layers.push(new_layer_handle);

        new_layer_id
    }

    ///
    /// Ensures that a particular layer exists
    ///
    #[inline]
    pub (crate) fn ensure_layer(&mut self, layer_id: canvas::LayerId) {
        // Add layers until we get to the current layer ID
        while self.ordered_layers.len() <= layer_id.0 as usize {
            self.create_new_layer();
        }
    }

    ///
    /// Retrieves the layer with the specified ID
    ///
    #[inline]
    pub (crate) fn layer_with_id(&mut self, layer_id: canvas::LayerId) -> Option<&mut Layer> {
        self.ordered_layers.get(layer_id.0 as usize)
            .copied()
            .and_then(move |layer_handle| self.layers.get_mut(layer_handle.0))
    }

    ///
    /// Retrieves the layer with a particular handle
    ///
    #[inline]
    pub (crate) fn layer(&mut self, layer_handle: LayerHandle) -> Option<&mut Layer> {
        self.layers.get_mut(layer_handle.0)
    }

    ///
    /// Retrieves the active layer
    ///
    #[inline]
    pub (crate) fn current_layer(&mut self) -> &mut Layer {
        self.layers.get_mut(self.current_layer.0).unwrap()
    }

    ///
    /// Selects or creates a layer with the given ID
    ///
    #[inline]
    pub (crate) fn select_layer(&mut self, layer_id: canvas::LayerId) {
        // Add layers until we get to the current layer ID
        self.ensure_layer(layer_id);

        // Pick this layer
        self.current_layer = self.ordered_layers[layer_id.0 as usize];
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
                alpha:          layer.alpha,
                blend_mode:     layer.blend_mode,
                edges:          layer.edges.clone(),
                used_data:      layer.used_data.clone(),
                stored_edges:   layer.stored_edges.clone(),
                stored_data:    layer.stored_data.clone(),
                z_index:        layer.z_index,
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

        if let Some(layer) = self.layer_with_id(layer_id) {
            layer.blend_mode = operation;
        }
    }

    ///
    /// Sets the alpha factor of a layer
    ///
    #[inline]
    pub (crate) fn layer_alpha(&mut self, layer_id: canvas::LayerId, alpha: f64) {
        self.ensure_layer(layer_id);

        if let Some(layer) = self.layer_with_id(layer_id) {
            layer.alpha = alpha;
        }
    }

    ///
    /// Clears all of the layers in the current drawing
    ///
    #[inline]
    pub (crate) fn clear_all_layers(&mut self) {
        let layers              = &mut self.layers;
        let program_data_cache  = &mut self.program_data_cache;

        layers.iter_mut()
            .for_each(|(_, layer)| {
                layer.clear();

                // Release the layer's data
                for data_id in layer.used_data.drain(..) {
                    program_data_cache.release_program_data(data_id);
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
