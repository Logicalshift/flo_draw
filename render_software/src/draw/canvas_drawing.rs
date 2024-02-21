use super::drawing_state::*;
use super::layer::*;
use super::prepared_layer::*;
use super::pixel_programs::*;
use super::texture::*;

use crate::edgeplan::*;
use crate::pixel::*;
use crate::pixel_programs::*;

use flo_sparse_array::*;

use flo_canvas as canvas;

use std::collections::{HashMap};
use std::sync::*;

///
/// A `CanvasDrawing` represents the state of a drawing after a series of `Draw` commands have been processed
///
pub struct CanvasDrawing<TPixel, const N: usize>
where
    TPixel: 'static + Send + Sync + Pixel<N>,
{
    /// The gamma correction value for the current drawing
    pub (super) gamma:              f64,

    /// The height in pixels of the target (used for things like line_width_pixels)
    pub (super) height_pixels:      f64,

    /// The program data ID for the program used to render the background
    pub (super) background:         PixelProgramDataId,

    /// The namespace for the current set of IDs
    pub (super) current_namespace:  canvas::NamespaceId,

    /// The layer that we're currently writing to
    pub (super) current_layer:      LayerHandle,

    /// The current drawing state
    pub (super) current_state:      DrawingState,

    /// Maps layer handles to layers
    pub (super) layers:             SparseArray<Layer>,

    /// The layers to render in order
    pub (super) ordered_layers:     Vec<LayerHandle>,

    /// For layers that have not been altered since they were last used by a sprite rendering command, the ready-to-render version
    pub (super) prepared_layers:    SparseArray<PreparedLayer>,

    /// The layer handles that map from sprite IDs
    pub (super) sprites:            HashMap<(canvas::NamespaceId, canvas::SpriteId), LayerHandle>,

    /// The next layer handle to allocate
    pub (super) next_layer_handle:  LayerHandle,

    /// Used to store the pixel programs used by this drawing
    pub (super) program_cache:      CanvasPixelPrograms<TPixel, N>,

    /// Used to store the data for the pixel program used by this drawing
    pub (super) program_data_cache: PixelProgramDataCache<TPixel>,

    /// States that have been pushed by PushState
    pub (super) state_stack:        Vec<DrawingState>,

    /// The textures in this drawing
    pub (super) textures:           HashMap<(canvas::NamespaceId, canvas::TextureId), Texture>,
}

impl<TPixel, const N: usize> CanvasDrawing<TPixel, N> 
where
    TPixel: 'static + Send + Sync + Pixel<N>,
{
    ///
    /// Creates a blank canvas drawing
    ///
    pub fn empty() -> Self {
        // Create an empty initial layer
        let mut layers = SparseArray::<Layer>::empty();
        let initial_layer = Layer::default();

        layers.insert(0, initial_layer);

        // Create the program and data cache
        let mut program_cache   = CanvasPixelPrograms::default();
        let mut data_cache      = program_cache.create_data_cache();

        // Default background colour is solid white
        let background          = program_cache.program_cache.store_program_data(&program_cache.solid_color, &mut data_cache, SolidColorData(TPixel::white()));

        CanvasDrawing {
            gamma:              2.2,
            height_pixels:      1080.0,
            background:         background,
            current_namespace:  canvas::NamespaceId::default(),
            current_layer:      LayerHandle(0),
            current_state:      DrawingState::default(),
            layers:             layers,
            prepared_layers:    SparseArray::empty(),
            ordered_layers:     vec![LayerHandle(0)],
            sprites:            HashMap::new(),
            next_layer_handle:  LayerHandle(1),
            program_cache:      program_cache,
            program_data_cache: data_cache,
            state_stack:        vec![],
            textures:           HashMap::new(),
        }
    }

    ///
    /// Sets the height in pixels of the target for this drawing
    ///
    /// (This is used for pixel-precise operations like `LineWidthPixels()`)
    ///
    pub fn set_pixel_height(&mut self, pixel_height: f64) {
        self.height_pixels = pixel_height;
    }

    ///
    /// Updates the state of this drawing with some drawing instructions
    ///
    pub fn draw(&mut self, drawing: impl IntoIterator<Item=canvas::Draw>) {
        for instruction in drawing {
            use canvas::Draw::*;

            match instruction {
                StartFrame                                          => { /* For flow control outside of the renderer */ },
                ShowFrame                                           => { /* For flow control outside of the renderer */ },
                ResetFrame                                          => { /* For flow control outside of the renderer */ },

                Namespace(namespace)                                => { self.current_namespace = namespace; },

                ClearCanvas(color)                                  => { self.clear_canvas(TPixel::from_color(color, self.gamma)); },
                Layer(layer_id)                                     => { self.select_layer(layer_id); },
                LayerBlend(layer_id, blend_mode)                    => { self.layer_blend(layer_id, blend_mode); },
                LayerAlpha(layer_id, alpha)                         => { self.layer_alpha(layer_id, alpha as f64); },
                ClearLayer                                          => { self.clear_layer(self.current_layer); },
                ClearAllLayers                                      => { self.clear_all_layers(); },
                SwapLayers(layer_1, layer_2)                        => { self.swap_layers(layer_1, layer_2); },

                Path(path_op)                                       => { self.current_state.path_op(path_op); },
                Fill                                                => { self.fill(); },
                Stroke                                              => { self.stroke(); },

                LineWidth(width)                                    => { self.current_state.line_width(width as _); },
                LineWidthPixels(width_pixels)                       => { self.current_state.line_width_pixels(width_pixels as _, self.height_pixels as _); },
                LineJoin(join_style)                                => { self.current_state.line_join(join_style); },
                LineCap(cap_style)                                  => { self.current_state.line_cap(cap_style); },
                NewDashPattern                                      => { /* todo!() - dash patterns not supported yet */ },
                DashLength(_dash_length)                            => { /* todo!() - dash patterns not supported yet */ },
                DashOffset(_dash_offset)                            => { /* todo!() - dash patterns not supported yet */ },
                FillColor(fill_color)                               => { self.current_state.fill_solid_color(fill_color, &mut self.program_data_cache); },
                FillTexture(texture, (x1, y1), (x2, y2))            => { self.fill_texture(texture, x1, y1, x2, y2); },
                FillGradient(gradient, (x1, y1), (x2, y2))          => { /* todo!() */ },
                FillTransform(transform)                            => { self.current_state.fill_transform(transform); },
                StrokeColor(stroke_color)                           => { self.current_state.stroke_solid_color(stroke_color, &mut self.program_data_cache); },
                WindingRule(winding_rule)                           => { self.current_state.winding_rule(winding_rule); },
                BlendMode(blend_mode)                               => { self.current_state.blend_mode(blend_mode, &mut self.program_data_cache); },

                IdentityTransform                                   => { self.current_state.identity_transform(); },
                CanvasHeight(height)                                => { self.current_state.canvas_height(height); },
                CenterRegion((x1, y1), (x2, y2))                    => { self.current_state.center_region((x1, y1), (x2, y2)); },
                MultiplyTransform(transform)                        => { self.current_state.multiply_transform(transform); },

                Unclip                                              => { self.unclip(); },
                Clip                                                => { self.set_clipping_path(); },
                Store                                               => { self.store_layer_edges(); },
                Restore                                             => { self.restore_layer_edges(); },
                FreeStoredBuffer                                    => { self.free_stored_edges(); },
                PushState                                           => { self.push_state() },
                PopState                                            => { self.pop_state() },

                Sprite(sprite_id)                                   => { self.sprite(sprite_id); },
                MoveSpriteFrom(sprite_id)                           => { self.sprite_move_from(sprite_id); },
                ClearSprite                                         => { self.clear_layer(self.current_layer); },
                SpriteTransform(transform)                          => { self.current_state.sprite_transform(transform); },
                DrawSprite(sprite_id)                               => { self.sprite_draw(sprite_id); },
                DrawSpriteWithFilters(sprite_id, filters)           => { /* todo!() */ },

                Texture(texture_id, texture_op)                     => { self.texture(texture_id, texture_op); },
                Gradient(gradient_id, gradient_op)                  => { /* todo!() */ },

                Font(_font_id, _font_op)                            => { /* Use the glyph and font streams in flo_canvas */ },
                BeginLineLayout(_x, _y, _alignment)                 => { /* Use the glyph and font streams in flo_canvas */ },
                DrawLaidOutText                                     => { /* Use the glyph and font streams in flo_canvas */ },
                DrawText(_font_id, _text, _x, _y)                   => { /* Use the glyph and font streams in flo_canvas */ },
            }
        }

        // TODO: really want to defer this until we get to the point where we are actually planning to render something
        // (It's more efficient to only call this immediately before a render, in case there are things on the canvas that are never ultimately rendered)
        self.prepare_to_render();
    }

    ///
    /// Prepares the layers in this drawing for rendering
    ///
    #[cfg(feature="multithreading")]
    fn prepare_to_render(&mut self) {
        use rayon::prelude::*;

        let mut layers = self.layers.iter_mut()
            .map(|(_, layer)| layer)
            .collect::<Vec<_>>();

        // Prepare each layer for rendering
        layers.par_iter_mut()
            .for_each(|layer| layer.edges.prepare_to_render());
    }

    ///
    /// Prepares the layers in this drawing for rendering
    ///
    #[cfg(not(feature="multithreading"))]
    fn prepare_to_render(&mut self) {
        // Prepare each layer for rendering
        self.layers.iter_mut()
            .for_each(|(_, layer)| layer.edges.prepare_to_render());
    }

    ///
    /// Returns a program runner for this canvas drawing for a certain pixel size, determined from the height of the render target in pixels
    ///
    /// Note that `set_pixel_height()` is used for the line widths, and this pixel height is used for choosing the shader programs. These
    /// values are typically the same, but when rendering a scaled image, this value should be the real render height and the value set in
    /// `set_pixel_height()` should be the 'original' height. Ie, if rendering an image scaled for 1080p at 4k resolution, `set_pixel_height()`
    /// should be called with 1080 as the value, and this should be called with 2160.
    ///
    pub fn program_runner<'a>(&'a self, height_pixels: f64) -> impl 'a + PixelProgramRunner<TPixel = TPixel> {
        // The y-position for the scene goes from -1 to 1 so the pixel size is 2.0/height
        let pixel_size = 2.0 / height_pixels;

        self.program_data_cache.create_program_runner(PixelSize(pixel_size))
    }

    ///
    /// Clears the canvas
    ///
    pub (super) fn clear_canvas(&mut self, new_background_color: TPixel) {
        // Clear the state stack
        while self.state_stack.len() > 0 {
            self.pop_state();
        }

        // Create an empty set of layers, containing only layer 0
        let mut layers = SparseArray::<Layer>::empty();
        let initial_layer = Layer::default();

        layers.insert(0, initial_layer);

        self.current_state.release_all_programs(&mut self.program_data_cache);

        // Reset the state of the canvas
        self.current_layer      = LayerHandle(0);
        self.layers             = layers;
        self.current_state      = DrawingState::default();
        self.sprites            = HashMap::new();
        self.ordered_layers     = vec![LayerHandle(0)];
        self.current_namespace  = canvas::NamespaceId::default();
        self.next_layer_handle  = LayerHandle(1);
        self.textures           = HashMap::new();

        // Free the old program data
        self.program_data_cache.free_all_data();

        // Create a new background colour
        let background = self.program_cache.program_cache.store_program_data(&self.program_cache.solid_color, &mut self.program_data_cache, SolidColorData(new_background_color));
        self.background = background;
    }

    ///
    /// Returns the edge plan for a layer in this drawing, if that layer has a plan
    ///
    /// This can be used for manual rendering or other types of post-processing beyond the capabilities of `CanvasDrawingRegionRenderer`
    ///
    pub fn edges_for_layer<'a>(&'a self, layer_id: canvas::LayerId) -> Option<&'a EdgePlan<Arc<dyn EdgeDescriptor>>> {
        // Map the layer to a layer handle, if it exists
        let layer_handle = self.ordered_layers.get(layer_id.0 as usize).copied()?;

        // Retrieve the edges for the layer with this handle
        self.layers.get(layer_handle.0 as _)
            .map(|layer| &layer.edges)
    }
}
