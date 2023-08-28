use super::drawing_state::*;
use super::layer::*;

use flo_sparse_array::*;

use flo_canvas as canvas;

///
/// A `CanvasDrawing` represents the state of a drawing after a series of `Draw` commands have been processed
///
pub struct CanvasDrawing {
    /// The namespace for the current set of IDs
    current_namespace:  canvas::NamespaceId,

    /// The layer that we're currently writing to
    current_layer:      canvas::LayerId,

    /// The current drawing state
    current_state:      DrawingState,

    /// Maps layer handles to layers
    layers:             SparseArray<Layer>,

    /// The layers in order
    ordered_layers:     Vec<LayerHandle>,

    /// The next layer handle to allocate
    next_layer_handle:  LayerHandle,
}

impl CanvasDrawing {
    ///
    /// Creates a blank canvas drawing
    ///
    pub fn empty() -> Self {
        CanvasDrawing {
            current_namespace:  canvas::NamespaceId::default(),
            current_layer:      canvas::LayerId(0),
            current_state:      DrawingState::default(),
            layers:             SparseArray::empty(),
            ordered_layers:     vec![],
            next_layer_handle:  LayerHandle(0),
        }
    }

    ///
    /// Updates the state of this drawing with some drawing instructions
    ///
    pub fn draw(&mut self, drawing: impl IntoIterator<Item=canvas::Draw>) {
        for instruction in drawing {
            use canvas::Draw::*;

            match instruction {
                StartFrame                                          => { todo!() },
                ShowFrame                                           => { todo!() },
                ResetFrame                                          => { todo!() },
                Namespace(namespace)                                => { todo!() },

                Path(path_op)                                       => { todo!() },
                Fill                                                => { todo!() },
                Stroke                                              => { todo!() },

                LineWidth(width)                                    => { todo!() },
                LineWidthPixels(width_pixels)                       => { todo!() },
                LineJoin(join_style)                                => { todo!() },
                LineCap(cap_style)                                  => { todo!() },
                NewDashPattern                                      => { todo!() },
                DashLength(dash_length)                             => { todo!() },
                DashOffset(dash_offset)                             => { todo!() },
                FillColor(fill_color)                               => { todo!() },
                FillTexture(texture, (x1, y1), (x2, y2))            => { todo!() },
                FillGradient(gradient, (x1, y1), (x2, y2))          => { todo!() },
                FillTransform(transform)                            => { todo!() },
                StrokeColor(color)                                  => { todo!() },
                WindingRule(winding_rule)                           => { todo!() },
                BlendMode(blend_mode)                               => { todo!() },

                IdentityTransform                                   => { todo!() },
                CanvasHeight(height)                                => { todo!() },
                CenterRegion((x1, y1), (x2, y2))                    => { todo!() },
                MultiplyTransform(transform)                        => { todo!() },

                Unclip                                              => { todo!() },
                Clip                                                => { todo!() },
                Store                                               => { todo!() },
                Restore                                             => { todo!() },
                FreeStoredBuffer                                    => { todo!() },
                PushState                                           => { todo!() },
                PopState                                            => { todo!() },

                ClearCanvas(color)                                  => { todo!() },
                Layer(layer_id)                                     => { todo!() },
                LayerBlend(layer_id, blend_mode)                    => { todo!() },
                LayerAlpha(layer_id, alpha)                         => { todo!() },
                ClearLayer                                          => { todo!() },
                ClearAllLayers                                      => { todo!() },
                SwapLayers(layer_1, layer_2)                        => { todo!() },

                Sprite(sprite_id)                                   => { todo!() },
                MoveSpriteFrom(sprite_id)                           => { todo!() },
                ClearSprite                                         => { todo!() },
                SpriteTransform(transform)                          => { todo!() },
                DrawSprite(sprite_id)                               => { todo!() },
                DrawSpriteWithFilters(sprite_id, filters)           => { todo!() },

                Texture(texture_id, texture_op)                     => { todo!() },
                Gradient(gradient_id, gradient_op)                  => { todo!() },

                Font(font_id, font_op)                              => { todo!() },
                BeginLineLayout(x, y, alignment)                    => { todo!() },
                DrawLaidOutText                                     => { todo!() },
                DrawText(font_id, text, x, y)                       => { todo!() },
            }
        }
    }
}