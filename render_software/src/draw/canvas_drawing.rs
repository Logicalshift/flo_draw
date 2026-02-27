use super::drawing_state::*;
use super::gradient::*;
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

    /// The base transform applied before any others to this drawing
    pub (super) base_transform:     canvas::Transform2D,

    /// The program data ID for the program used to render the background
    pub (super) background:         PixelProgramDataId,

    /// The namespace for the current set of IDs
    pub (super) current_namespace:  usize,

    /// The layer that we're currently writing to
    pub (super) current_layer:      LayerHandle,

    /// The current drawing state
    pub (super) current_state:      DrawingState,

    /// Maps layer handles to layers
    pub (super) layers:             SparseArray<Layer>,

    /// Maps layer IDs to handles
    pub (super) handle_for_layer:   HashMap<(usize, canvas::LayerId), LayerHandle>,

    /// The layers to render in order
    pub (super) ordered_layers:     Vec<LayerHandle>,

    /// For layers that have not been altered since they were last used by a sprite rendering command, the ready-to-render version
    pub (super) prepared_layers:    SparseArray<PreparedLayer>,

    /// The layer handles that map from sprite IDs
    pub (super) sprites:            HashMap<(usize, canvas::SpriteId), LayerHandle>,

    /// The next layer handle to allocate
    pub (super) next_layer_handle:  LayerHandle,

    /// Used to store the pixel programs used by this drawing
    pub (super) program_cache:      CanvasPixelPrograms<TPixel, N>,

    /// Used to store the data for the pixel program used by this drawing
    pub (super) program_data_cache: PixelProgramDataCache<TPixel>,

    /// States that have been pushed by PushState
    pub (super) state_stack:        Vec<DrawingState>,

    /// The textures in this drawing (usize is the namespace local ID)
    pub (super) textures:           HashMap<(usize, canvas::TextureId), Texture>,

    /// The gradients in this drawing (usize is the namespace local ID)
    pub (super) gradients:          HashMap<(usize, canvas::GradientId), Gradient<TPixel>>,
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

        let mut handle_for_layer = HashMap::new();
        handle_for_layer.insert((canvas::NamespaceId::default().local_id(), canvas::LayerId(0)), LayerHandle(0));

        CanvasDrawing {
            gamma:              2.2,
            height_pixels:      1080.0,
            base_transform:     canvas::Transform2D::scale(1.0, -1.0),
            background:         background,
            current_namespace:  canvas::NamespaceId::default().local_id(),
            current_layer:      LayerHandle(0),
            current_state:      DrawingState::default(),
            layers:             layers,
            handle_for_layer:   handle_for_layer,
            prepared_layers:    SparseArray::empty(),
            ordered_layers:     vec![LayerHandle(0)],
            sprites:            HashMap::new(),
            next_layer_handle:  LayerHandle(1),
            program_cache:      program_cache,
            program_data_cache: data_cache,
            state_stack:        vec![],
            textures:           HashMap::new(),
            gradients:          HashMap::new(),
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
    /// Sets the base transformation for this canvas (eg, to flip the image over)
    ///
    pub fn set_base_transform(&mut self, transform: canvas::Transform2D) {
        self.base_transform = transform;
    }

    ///
    /// Retrieves the base transform for this canvas
    ///
    pub fn base_transform(&self) -> canvas::Transform2D {
        self.base_transform
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

                Namespace(namespace)                                => { self.current_namespace = namespace.local_id(); },

                ClearCanvas(color)                                  => { self.clear_canvas(TPixel::from_color(color, self.gamma)); },
                Layer(layer_id)                                     => { self.select_layer(layer_id); },
                LayerBlend(layer_id, blend_mode)                    => { self.layer_blend(layer_id, blend_mode); },
                LayerAlpha(layer_id, alpha)                         => { self.layer_alpha(layer_id, alpha as f64); },
                ClearLayer                                          => { self.clear_layer(self.current_layer); },
                ClearAllLayers                                      => { self.clear_all_layers(); },
                SwapLayers(layer_1, layer_2)                        => { self.swap_layers(layer_1, layer_2); },
                PlaceLayerBefore(namespace_id, layer_id)            => { self.place_layer_before(namespace_id, layer_id); }
                SetLayerTransform(layer_transform)                  => { self.set_layer_transform(layer_transform); }

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
                FillGradient(gradient, (x1, y1), (x2, y2))          => { self.fill_gradient(gradient, x1, y1, x2, y2); },
                FillTransform(transform)                            => { self.current_state.fill_transform(transform); },
                StrokeColor(stroke_color)                           => { self.current_state.stroke_solid_color(stroke_color, &mut self.program_data_cache); },
                WindingRule(winding_rule)                           => { self.current_state.winding_rule(winding_rule); },
                BlendMode(blend_mode)                               => { self.current_state.blend_mode(blend_mode, &mut self.program_data_cache); },

                IdentityTransform                                   => { self.current_state.identity_transform(&self.base_transform); },
                CanvasHeight(height)                                => { self.current_state.canvas_height(height, &self.base_transform); },
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
                DrawSpriteWithFilters(sprite_id, filters)           => { self.sprite_draw_with_filters(sprite_id, filters); },

                Texture(texture_id, texture_op)                     => { self.texture(texture_id, texture_op); },
                Gradient(gradient_id, gradient_op)                  => { self.gradient(gradient_id, gradient_op); },

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
            .for_each(|layer| {
                // TODO: we can avoid performing a transformation if the transform does not do a rotation (we can just change the position of where we get the intercepts later on)
                if layer.layer_transform.is_rotation() || layer.layer_transform != canvas::Transform2D::identity() {
                    // Free any existing transformed edges
                    layer.transformed_edges = None;

                    // Transform the edges according to the layer transform
                    let mut transformed_edges = layer.edges.transform(&layer.layer_transform);
                    transformed_edges.prepare_to_render();

                    layer.transformed_edges = Some(transformed_edges);
                } else {
                    layer.edges.prepare_to_render();
                }
            });
    }

    ///
    /// Prepares the layers in this drawing for rendering
    ///
    #[cfg(not(feature="multithreading"))]
    fn prepare_to_render(&mut self) {
        // Prepare each layer for rendering
        self.layers.iter_mut()
            .for_each(|(_, layer)| {
                // TODO: we can avoid performing a transformation if the transform does not do a rotation (we can just change the position of where we get the intercepts later on)
                if layer.layer_transform.is_rotation() || layer.layer_transform != canvas::Transform2D::identity() {
                    // Free any existing transformed edges
                    layer.transformed_edges = None;

                    // Transform the edges according to the layer transform
                    let mut transformed_edges = layer.edges.transform(&layer.layer_transform);
                    transformed_edges.prepare_to_render();

                    layer.transformed_edges = Some(transformed_edges);
                } else {
                    layer.edges.prepare_to_render();
                }
            });
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
        self.handle_for_layer   = HashMap::new();
        self.current_state      = DrawingState::default();
        self.sprites            = HashMap::new();
        self.ordered_layers     = vec![LayerHandle(0)];
        self.current_namespace  = canvas::NamespaceId::default().local_id();
        self.next_layer_handle  = LayerHandle(1);
        self.textures           = HashMap::new();

        self.handle_for_layer.insert((canvas::NamespaceId::default().local_id(), canvas::LayerId(0)), LayerHandle(0));

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
    pub fn edges_for_layer<'a>(&'a self, namespace_id: canvas::NamespaceId, layer_id: canvas::LayerId) -> Option<&'a EdgePlan<Arc<dyn EdgeDescriptor>>> {
        // Map the layer to a layer handle, if it exists
        let layer = self.layer_with_id_readonly(namespace_id.local_id(), layer_id);

        layer.map(|layer| &layer.edges)
    }

    ///
    /// Retrieves the 'active' transform that maps canvas coordinates to the
    /// (-1, 1) range used for the 'source' coordinate scheme
    ///
    pub fn active_transform(&self) -> canvas::Transform2D {
        self.current_state.transform
    }
}

#[cfg(test)]
mod namespace_order_tests {
    use super::*;
    use flo_canvas as canvas;
    use crate::pixel::F32LinearPixel;

    type TestDrawing = CanvasDrawing<F32LinearPixel, 4>;

    ///
    /// Returns the position of a namespace's layer within `ordered_layers`, if it exists
    ///
    fn position_of(drawing: &TestDrawing, namespace: canvas::NamespaceId, layer_id: canvas::LayerId) -> Option<usize> {
        let handle = drawing.handle_for_layer.get(&(namespace.local_id(), layer_id))?;
        drawing.ordered_layers.iter().position(|h| h == handle)
    }

    ///
    /// Creates a drawing containing five namespaces in a defined order, matching the structure
    /// from the user's drawing instructions:
    ///
    ///   default -> canvas_ns -> physics_ns -> dock_ns -> dialog_ns
    ///
    /// The last two instructions switch back to the default namespace, exercising the
    /// "don't move an already-placed namespace" code path.
    ///
    fn make_five_namespace_drawing() -> (TestDrawing, canvas::NamespaceId, canvas::NamespaceId, canvas::NamespaceId, canvas::NamespaceId) {
        let canvas_ns   = canvas::NamespaceId::new();
        let physics_ns  = canvas::NamespaceId::new();
        let dock_ns     = canvas::NamespaceId::new();
        let dialog_ns   = canvas::NamespaceId::new();

        let mut drawing = CanvasDrawing::<F32LinearPixel, 4>::empty();

        drawing.draw([
            canvas::Draw::ClearCanvas(canvas::Color::Rgba(0.8, 0.8, 0.8, 1.0)),
            canvas::Draw::CanvasHeight(1024.0),

            canvas::Draw::Namespace(canvas::NamespaceId::default()),
            canvas::Draw::Layer(canvas::LayerId(0)),
            canvas::Draw::ClearLayer,

            canvas::Draw::Namespace(canvas_ns),
            canvas::Draw::Layer(canvas::LayerId(0)),
            canvas::Draw::ClearLayer,

            canvas::Draw::Namespace(physics_ns),
            canvas::Draw::Layer(canvas::LayerId(0)),
            canvas::Draw::ClearLayer,

            canvas::Draw::Namespace(dock_ns),
            canvas::Draw::Layer(canvas::LayerId(0)),
            canvas::Draw::ClearLayer,

            canvas::Draw::Namespace(dialog_ns),
            canvas::Draw::Layer(canvas::LayerId(0)),
            canvas::Draw::ClearLayer,

            // These last two lines switch back to the default namespace without adding a new one
            canvas::Draw::Namespace(canvas::NamespaceId::default()),
            canvas::Draw::Layer(canvas::LayerId(0)),
        ]);

        (drawing, canvas_ns, physics_ns, dock_ns, dialog_ns)
    }

    #[test]
    fn namespace_layers_appear_in_first_encounter_order() {
        // After processing the drawing instructions, the ordered_layers list should reflect
        // the order in which each namespace's layer 0 was first encountered:
        //   default < canvas_ns < physics_ns < dock_ns < dialog_ns
        let (drawing, canvas_ns, physics_ns, dock_ns, dialog_ns) = make_five_namespace_drawing();

        let default_pos = position_of(&drawing, canvas::NamespaceId::default(), canvas::LayerId(0))
            .expect("default namespace layer 0 must exist in ordered_layers");
        let canvas_pos  = position_of(&drawing, canvas_ns, canvas::LayerId(0))
            .expect("canvas namespace layer 0 must exist in ordered_layers");
        let physics_pos = position_of(&drawing, physics_ns, canvas::LayerId(0))
            .expect("physics namespace layer 0 must exist in ordered_layers");
        let dock_pos    = position_of(&drawing, dock_ns, canvas::LayerId(0))
            .expect("dock namespace layer 0 must exist in ordered_layers");
        let dialog_pos  = position_of(&drawing, dialog_ns, canvas::LayerId(0))
            .expect("dialog namespace layer 0 must exist in ordered_layers");

        assert!(default_pos < canvas_pos,
            "default namespace (pos {}) should appear before canvas namespace (pos {})",
            default_pos, canvas_pos);
        assert!(canvas_pos < physics_pos,
            "canvas namespace (pos {}) should appear before physics namespace (pos {})",
            canvas_pos, physics_pos);
        assert!(physics_pos < dock_pos,
            "physics namespace (pos {}) should appear before dock namespace (pos {})",
            physics_pos, dock_pos);
        assert!(dock_pos < dialog_pos,
            "dock namespace (pos {}) should appear before dialog namespace (pos {})",
            dock_pos, dialog_pos);
    }

    #[test]
    fn reselecting_namespace_does_not_change_layer_order() {
        // Running the same sequence of namespace selections a second time (simulating a second
        // frame) must not reorder any of the namespaces.
        let (mut drawing, canvas_ns, physics_ns, dock_ns, dialog_ns) = make_five_namespace_drawing();

        let before_default_pos  = position_of(&drawing, canvas::NamespaceId::default(), canvas::LayerId(0)).unwrap();
        let before_canvas_pos   = position_of(&drawing, canvas_ns,  canvas::LayerId(0)).unwrap();
        let before_physics_pos  = position_of(&drawing, physics_ns, canvas::LayerId(0)).unwrap();
        let before_dock_pos     = position_of(&drawing, dock_ns,    canvas::LayerId(0)).unwrap();
        let before_dialog_pos   = position_of(&drawing, dialog_ns,  canvas::LayerId(0)).unwrap();

        // Re-run the exact same sequence of drawing instructions (second frame, same namespaces)
        drawing.draw([
            canvas::Draw::Namespace(canvas::NamespaceId::default()),
            canvas::Draw::Layer(canvas::LayerId(0)),
            canvas::Draw::ClearLayer,

            canvas::Draw::Namespace(canvas_ns),
            canvas::Draw::Layer(canvas::LayerId(0)),
            canvas::Draw::ClearLayer,

            canvas::Draw::Namespace(physics_ns),
            canvas::Draw::Layer(canvas::LayerId(0)),
            canvas::Draw::ClearLayer,

            canvas::Draw::Namespace(dock_ns),
            canvas::Draw::Layer(canvas::LayerId(0)),
            canvas::Draw::ClearLayer,

            canvas::Draw::Namespace(dialog_ns),
            canvas::Draw::Layer(canvas::LayerId(0)),
            canvas::Draw::ClearLayer,

            canvas::Draw::Namespace(canvas::NamespaceId::default()),
            canvas::Draw::Layer(canvas::LayerId(0)),
        ]);

        let after_default_pos   = position_of(&drawing, canvas::NamespaceId::default(), canvas::LayerId(0)).unwrap();
        let after_canvas_pos    = position_of(&drawing, canvas_ns,  canvas::LayerId(0)).unwrap();
        let after_physics_pos   = position_of(&drawing, physics_ns, canvas::LayerId(0)).unwrap();
        let after_dock_pos      = position_of(&drawing, dock_ns,    canvas::LayerId(0)).unwrap();
        let after_dialog_pos    = position_of(&drawing, dialog_ns,  canvas::LayerId(0)).unwrap();

        assert_eq!(before_default_pos,  after_default_pos,  "default namespace must not move after second frame");
        assert_eq!(before_canvas_pos,   after_canvas_pos,   "canvas namespace must not move after second frame");
        assert_eq!(before_physics_pos,  after_physics_pos,  "physics namespace must not move after second frame");
        assert_eq!(before_dock_pos,     after_dock_pos,     "dock namespace must not move after second frame");
        assert_eq!(before_dialog_pos,   after_dialog_pos,   "dialog namespace must not move after second frame");
    }

    #[test]
    fn adding_higher_numbered_layer_inserts_within_namespace_not_after_next_namespace() {
        // When a new layer (e.g. Layer(1)) is added to an existing namespace, it should be
        // inserted immediately after that namespace's highest existing layer, and the layers
        // of subsequent namespaces must remain after it.
        let canvas_ns   = canvas::NamespaceId::new();
        let physics_ns  = canvas::NamespaceId::new();

        let mut drawing = CanvasDrawing::<F32LinearPixel, 4>::empty();

        drawing.draw([
            canvas::Draw::ClearCanvas(canvas::Color::Rgba(0.8, 0.8, 0.8, 1.0)),

            canvas::Draw::Namespace(canvas_ns),
            canvas::Draw::Layer(canvas::LayerId(0)),
            canvas::Draw::ClearLayer,

            canvas::Draw::Namespace(physics_ns),
            canvas::Draw::Layer(canvas::LayerId(0)),
            canvas::Draw::ClearLayer,
        ]);

        let canvas_layer0_pos_before    = position_of(&drawing, canvas_ns,  canvas::LayerId(0)).unwrap();
        let physics_layer0_pos_before   = position_of(&drawing, physics_ns, canvas::LayerId(0)).unwrap();

        // canvas_ns is before physics_ns at this point
        assert!(canvas_layer0_pos_before < physics_layer0_pos_before,
            "canvas/0 (pos {}) should be before physics/0 (pos {})",
            canvas_layer0_pos_before, physics_layer0_pos_before);

        // Now add Layer(1) to canvas_ns — it should sit between canvas/0 and physics/0
        drawing.draw([
            canvas::Draw::Namespace(canvas_ns),
            canvas::Draw::Layer(canvas::LayerId(1)),
        ]);

        let canvas_layer0_pos  = position_of(&drawing, canvas_ns,  canvas::LayerId(0)).unwrap();
        let canvas_layer1_pos  = position_of(&drawing, canvas_ns,  canvas::LayerId(1)).unwrap();
        let physics_layer0_pos = position_of(&drawing, physics_ns, canvas::LayerId(0)).unwrap();

        assert!(canvas_layer0_pos < canvas_layer1_pos,
            "canvas/Layer(0) (pos {}) should come before canvas/Layer(1) (pos {})",
            canvas_layer0_pos, canvas_layer1_pos);
        assert!(canvas_layer1_pos < physics_layer0_pos,
            "canvas/Layer(1) (pos {}) should come before physics/Layer(0) (pos {}), \
             but physics namespace appears to have been displaced",
            canvas_layer1_pos, physics_layer0_pos);
    }

    #[test]
    fn clear_layer_does_not_reorder_namespaces() {
        // ClearLayer clears the contents of a layer but must not move it within ordered_layers
        let (mut drawing, canvas_ns, physics_ns, _dock_ns, _dialog_ns) = make_five_namespace_drawing();

        let before_default  = position_of(&drawing, canvas::NamespaceId::default(), canvas::LayerId(0)).unwrap();
        let before_canvas   = position_of(&drawing, canvas_ns,  canvas::LayerId(0)).unwrap();
        let before_physics  = position_of(&drawing, physics_ns, canvas::LayerId(0)).unwrap();

        // Clear the middle namespace's layer
        drawing.draw([
            canvas::Draw::Namespace(canvas_ns),
            canvas::Draw::Layer(canvas::LayerId(0)),
            canvas::Draw::ClearLayer,
        ]);

        let after_default   = position_of(&drawing, canvas::NamespaceId::default(), canvas::LayerId(0)).unwrap();
        let after_canvas    = position_of(&drawing, canvas_ns,  canvas::LayerId(0)).unwrap();
        let after_physics   = position_of(&drawing, physics_ns, canvas::LayerId(0)).unwrap();

        assert_eq!(before_default, after_default,   "default namespace position must not change after ClearLayer");
        assert_eq!(before_canvas,  after_canvas,    "canvas namespace position must not change after ClearLayer");
        assert_eq!(before_physics, after_physics,   "physics namespace position must not change after ClearLayer");
    }
}
