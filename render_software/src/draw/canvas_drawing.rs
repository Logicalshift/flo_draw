use super::drawing_state::*;
use super::layer::*;
use super::path::*;
use super::pixel_programs::*;

use crate::pixel::*;
use crate::pixel_programs::*;

use canvas::NamespaceId;
use flo_sparse_array::*;

use flo_canvas as canvas;

///
/// A `CanvasDrawing` represents the state of a drawing after a series of `Draw` commands have been processed
///
pub struct CanvasDrawing<TPixel, const N: usize>
where
    TPixel: 'static + Send + Sync + Pixel<N>,
{
    /// The gamma correction value for the current drawing
    pub (super) gamma:              f64,

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

    /// The layers in order
    pub (super) ordered_layers:     Vec<LayerHandle>,

    /// The next layer handle to allocate
    pub (super) next_layer_handle:  LayerHandle,

    /// Used to store the pixel programs used by this drawing
    pub (super) program_cache:      CanvasPixelPrograms<TPixel, N>,

    /// Used to store the data for the pixel program used by this drawing
    pub (super) program_data_cache: PixelProgramDataCache<TPixel>,

    /// States that have been pushed by PushState
    pub (super) state_stack:        Vec<DrawingState>,
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
            background:         background,
            current_namespace:  canvas::NamespaceId::default(),
            current_layer:      LayerHandle(0),
            current_state:      DrawingState::default(),
            layers:             layers,
            ordered_layers:     vec![LayerHandle(0)],
            next_layer_handle:  LayerHandle(1),
            program_cache:      program_cache,
            program_data_cache: data_cache,
            state_stack:        vec![],
        }
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

                LineWidth(width)                                    => { /* todo!() */ },
                LineWidthPixels(width_pixels)                       => { /* todo!() */ },
                LineJoin(join_style)                                => { /* todo!() */ },
                LineCap(cap_style)                                  => { /* todo!() */ },
                NewDashPattern                                      => { /* todo!() */ },
                DashLength(dash_length)                             => { /* todo!() */ },
                DashOffset(dash_offset)                             => { /* todo!() */ },
                FillColor(fill_color)                               => { self.current_state.fill_solid_color(fill_color, &mut self.program_data_cache); },
                FillTexture(texture, (x1, y1), (x2, y2))            => { /* todo!() */ },
                FillGradient(gradient, (x1, y1), (x2, y2))          => { /* todo!() */ },
                FillTransform(transform)                            => { /* todo!() */ },
                StrokeColor(stroke_color)                           => { self.current_state.stroke_solid_color(stroke_color, &mut self.program_data_cache); },
                WindingRule(winding_rule)                           => { self.current_state.winding_rule(winding_rule); },
                BlendMode(blend_mode)                               => { /* todo!() */ },

                IdentityTransform                                   => { self.current_state.identity_transform(); },
                CanvasHeight(height)                                => { self.current_state.canvas_height(height); },
                CenterRegion((x1, y1), (x2, y2))                    => { self.current_state.center_region((x1, y1), (x2, y2)); },
                MultiplyTransform(transform)                        => { self.current_state.multiply_transform(transform); },

                Unclip                                              => { /* todo!() */ },
                Clip                                                => { /* todo!() */ },
                Store                                               => { /* todo!() */ },
                Restore                                             => { /* todo!() */ },
                FreeStoredBuffer                                    => { /* todo!() */ },
                PushState                                           => { self.push_state() },
                PopState                                            => { self.pop_state() },

                Sprite(sprite_id)                                   => { /* todo!() */ },
                MoveSpriteFrom(sprite_id)                           => { /* todo!() */ },
                ClearSprite                                         => { /* todo!() */ },
                SpriteTransform(transform)                          => { /* todo!() */ },
                DrawSprite(sprite_id)                               => { /* todo!() */ },
                DrawSpriteWithFilters(sprite_id, filters)           => { /* todo!() */ },

                Texture(texture_id, texture_op)                     => { /* todo!() */ },
                Gradient(gradient_id, gradient_op)                  => { /* todo!() */ },

                Font(font_id, font_op)                              => { /* todo!() */ },
                BeginLineLayout(x, y, alignment)                    => { /* todo!() */ },
                DrawLaidOutText                                     => { /* todo!() */ },
                DrawText(font_id, text, x, y)                       => { /* todo!() */ },
            }
        }
    }

    ///
    /// Returns the program runner for this canvas drawing
    ///
    pub fn program_runner(&self) -> &PixelProgramDataCache<TPixel> {
        &self.program_data_cache
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
        self.ordered_layers     = vec![LayerHandle(0)];
        self.current_namespace  = NamespaceId::default();
        self.next_layer_handle  = LayerHandle(1);

        // Free the old program data
        self.program_data_cache.free_all_data();

        // Create a new background colour
        let background = self.program_cache.program_cache.store_program_data(&self.program_cache.solid_color, &mut self.program_data_cache, SolidColorData(new_background_color));
        self.background = background;
    }
}
