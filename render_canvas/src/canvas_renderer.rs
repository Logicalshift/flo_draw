use super::tessellate::*;
use super::stroke_settings::*;

use flo_render as render;
use flo_canvas as canvas;
use flo_stream::*;

use ::desync::*;

use futures::prelude::*;
use num_cpus;
use lyon::tessellation::{VertexBuffers};
use lyon::path;
use lyon::math;

use std::sync::*;

///
/// Operation to use when drawing an item on a layer
///
#[derive(Clone, Copy)]
enum LayerOperation {
    /// Draw the vertex buffer
    Draw,

    /// Erase the vertex buffer
    Erase
}

///
/// Single rendering operation for a layer
///
enum RenderEntity {
    /// Render operation is waiting to be tessellated
    Tessellating(LayerOperation),

    /// Tessellation waiting to be sent to the renderer
    VertexBuffer(LayerOperation, VertexBuffers<render::Vertex2D, u16>),

    /// Render a vertex buffer
    DrawIndexed(LayerOperation, render::VertexBufferId, render::VertexBufferId)
}

///
/// Definition of a layer in the canvas
///
struct Layer {
    /// The render order for this layer
    render_order: Vec<RenderEntity>,

    /// The current fill colour
    fill_color: render::Rgba8,

    /// The settings for the next brush stroke
    stroke_settings: StrokeSettings
}

///
/// Changes commands for `flo_canvas` into commands for `flo_render`
///
pub struct CanvasRenderer {
    /// The worker threads
    workers: Vec<Arc<Desync<Tessellator>>>,

    /// Layers defined by the canvas
    layers: Vec<Layer>,

    /// The layer that the next drawing instruction will apply to
    current_layer: usize
}

impl CanvasRenderer {
    ///
    /// Creates a new canvas renderer
    ///
    pub fn new() -> CanvasRenderer {
        // Create one worker per cpu
        let num_workers = num_cpus::get();
        let mut workers = Vec::with_capacity(num_workers);

        for _ in 0..num_workers {
            workers.push(Arc::new(Desync::new(Tessellator::new())));
        }

        // Generate the final renderer
        CanvasRenderer {
            workers:        workers,
            layers:         vec![],
            current_layer:  0
        }
    }

    ///
    /// Creates a new layer with the default properties
    ///
    fn create_default_layer(&self) -> Layer {
        Layer {
            render_order:       vec![],
            fill_color:         render::Rgba8([0, 0, 0, 255]),
            stroke_settings:    StrokeSettings::new()
        }
    }

    ///
    /// Tessellates a drawing to the layers in this renderer
    ///
    fn tessellate<'a, DrawIter: 'a+Iterator<Item=canvas::Draw>>(&'a mut self, drawing: DrawIter) -> impl 'a+Future<Output=()> {
        async move {
            // The current path that is being built up
            let mut path_builder = None;

            // The last path that was generated
            let mut current_path = None;

            // Create the default layer if one doesn't already exist
            if self.layers.len() == 0 {
                self.current_layer  = 0;
                self.layers         = vec![self.create_default_layer()];
            }

            // Iterate through the drawing instructions
            for draw in drawing {
                use canvas::Draw::*;
                use math::point;

                match draw {
                    // Begins a new path
                    NewPath => {
                        current_path = None;
                        path_builder = Some(path::Builder::new());
                    }

                    // Move to a new point
                    Move(x, y) => {
                        path_builder.get_or_insert_with(|| path::Builder::new())
                            .move_to(point(x, y));
                    }

                    // Line to point
                    Line(x, y) => {
                        path_builder.get_or_insert_with(|| path::Builder::new())
                            .line_to(point(x, y));
                    }

                    // Bezier curve to point
                    BezierCurve((px, py), (cp1x, cp1y), (cp2x, cp2y)) => {
                        path_builder.get_or_insert_with(|| path::Builder::new())
                            .cubic_bezier_to(point(cp1x, cp1y), point(cp2x, cp2y), point(px, py));
                    }

                    // Closes the current path
                    ClosePath => {
                        path_builder.get_or_insert_with(|| path::Builder::new())
                            .close();
                    }

                    // Fill the current path
                    Fill => {
                        // Update the active path if the builder exists
                        if let Some(path_builder) = path_builder.take() {
                            current_path = Some(path_builder.build());
                        }

                        unimplemented!()
                    }

                    // Draw a line around the current path
                    Stroke => {
                        // Update the active path if the builder exists
                        if let Some(path_builder) = path_builder.take() {
                            current_path = Some(path_builder.build());
                        }

                        unimplemented!()
                    }

                    // Set the line width
                    LineWidth(f32) => {
                        unimplemented!()
                    }

                    // Set the line width in pixels
                    LineWidthPixels(f32) => {
                        unimplemented!()
                    }

                    // Line join
                    LineJoin(join_type) => {
                        unimplemented!()
                    }

                    // The cap to use on lines
                    LineCap(cap_type) => {
                        unimplemented!()
                    }

                    // Resets the dash pattern to empty (which is a solid line)
                    NewDashPattern => {
                        unimplemented!()
                    }

                    // Adds a dash to the current dash pattern
                    DashLength(f32) => {
                        unimplemented!()
                    }

                    // Sets the offset for the dash pattern
                    DashOffset(f32) => {
                        unimplemented!()
                    }

                    // Set the fill color
                    FillColor(Color) => {
                        unimplemented!()
                    }

                    /// Set the line color
                    StrokeColor(Color) => {
                        unimplemented!()
                    }

                    // Set how future renderings are blended with one another
                    BlendMode(blend_mode) => {
                        unimplemented!()
                    }

                    // Reset the transformation to the identity transformation
                    IdentityTransform => {
                        unimplemented!()
                    }

                    // Sets a transformation such that:
                    // (0,0) is the center point of the canvas
                    // (0,height/2) is the top of the canvas
                    // Pixels are square
                    CanvasHeight(height) => {
                        unimplemented!()
                    }

                    // Moves a particular region to the center of the canvas (coordinates are minx, miny, maxx, maxy)
                    CenterRegion((x1, y1), (x2, y2)) => {
                        unimplemented!()
                    }

                    // Multiply a 2D transform into the canvas
                    MultiplyTransform(transform) => {
                        unimplemented!()
                    }

                    // Unset the clipping path
                    Unclip => {
                        unimplemented!()
                    }

                    // Clip to the currently set path
                    Clip => {
                        unimplemented!()
                    }

                    // Stores the content of the clipping path from the current layer in a background buffer
                    Store => {
                        unimplemented!()
                    }

                    // Restores what was stored in the background buffer. This should be done on the
                    // same layer that the Store operation was called upon.
                    //
                    // The buffer is left intact by this operation so it can be restored again in the future.
                    //
                    // (If the clipping path has changed since then, the restored image is clipped against the new path)
                    Restore => {
                        unimplemented!()
                    }

                    // Releases the buffer created by the last 'Store' operation
                    //
                    // Restore will no longer be valid for the current layer
                    FreeStoredBuffer => {
                        unimplemented!()
                    }

                    // Push the current state of the canvas (line settings, stored image, current path - all state)
                    PushState => {
                        unimplemented!()
                    }

                    // Restore a state previously pushed
                    PopState => {
                        unimplemented!()
                    }

                    // Clears the canvas entirely
                    ClearCanvas => {
                        unimplemented!()
                    }

                    // Selects a particular layer for drawing
                    // Layer 0 is selected initially. Layers are drawn in order starting from 0.
                    // Layer IDs don't have to be sequential.
                    Layer(layer_id) => {
                        unimplemented!()
                    }

                    // Sets how a particular layer is blended with the underlying layer
                    LayerBlend(layer_id, blend_mode) => {
                        unimplemented!()
                    }

                    // Clears the current layer
                    ClearLayer => {
                        unimplemented!()
                    }
                }
            }
        }
    }

    ///
    /// Returns a stream of render actions after applying a set of canvas drawing operations to this renderer
    ///
    pub fn draw<'a, DrawIter: 'a+Iterator<Item=canvas::Draw>>(&mut self, drawing: DrawIter) -> impl 'a+Stream<Item=render::RenderAction> {
        futures::stream::empty()
    }
}