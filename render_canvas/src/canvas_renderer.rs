use super::stroke_settings::*;
use super::renderer_core::*;
use super::renderer_layer::*;
use super::renderer_worker::*;

use flo_render as render;
use flo_canvas as canvas;
use flo_stream::*;

use ::desync::*;

use futures::prelude::*;
use futures::task::{Context, Poll};
use futures::future::{LocalBoxFuture};
use num_cpus;
use lyon::path;
use lyon::math;

use std::pin::*;
use std::sync::*;

///
/// Changes commands for `flo_canvas` into commands for `flo_render`
///
pub struct CanvasRenderer {
    /// The worker threads
    workers: Vec<Arc<Desync<CanvasWorker>>>,

    /// Layers defined by the canvas
    core: Arc<Desync<RenderCore>>,

    /// The layer that the next drawing instruction will apply to
    current_layer: usize,
}

impl CanvasRenderer {
    ///
    /// Creates a new canvas renderer
    ///
    pub fn new() -> CanvasRenderer {
        // Create the shared core
        let core = RenderCore {
            layers:                 vec![],
            unused_vertex_buffer:   0,
            free_vertex_buffers:    vec![]
        };
        let core = Arc::new(Desync::new(core));

        // Create one worker per cpu
        let num_workers = num_cpus::get();
        let mut workers = Vec::with_capacity(num_workers);

        for _ in 0..num_workers {
            workers.push(Arc::new(Desync::new(CanvasWorker::new(&core))));
        }

        // Generate the final renderer
        CanvasRenderer {
            workers:        workers,
            core:           core,
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
    /// Changes a colour component to a u8 format
    ///
    fn col_to_u8(component: f32) -> u8 {
        if component > 1.0 {
            255
        } else if component < 0.0 {
            0
        } else {
            (component * 255.0) as u8
        }
    }

    ///
    /// Converts a canvas colour to a render colour
    ///
    fn render_color(color: canvas::Color) -> render::Rgba8 {
        let (r, g, b, a)    = color.to_rgba_components();
        let (r, g, b, a)    = (Self::col_to_u8(r), Self::col_to_u8(g), Self::col_to_u8(b), Self::col_to_u8(a));

        render::Rgba8([r, g, b, a])
    }

    ///
    /// Tessellates a drawing to the layers in this renderer
    ///
    fn tessellate<'a, DrawIter: 'a+Iterator<Item=canvas::Draw>>(&'a mut self, drawing: DrawIter, job_publisher: SinglePublisher<CanvasJob>) -> impl 'a+Future<Output=()> {
        async move {
            let core                = Arc::clone(&self.core);
            let mut job_publisher   = job_publisher;

            // The current path that is being built up
            let mut path_builder = None;

            // The last path that was generated
            let mut current_path = None;

            // Create the default layer if one doesn't already exist
            core.sync(|core| {
                if core.layers.len() == 0 {
                    core.layers         = vec![self.create_default_layer()];
                    self.current_layer  = 0;
                }
            });

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

                        // Publish the fill job to the tessellators
                        if let Some(path) = &current_path {
                            let path        = path.clone();
                            let layer_id    = self.current_layer;

                            let job         = core.sync(move |core| {
                                // Create the render entity in the tessellating state
                                let color           = core.layers[layer_id].fill_color;
                                let entity_index    = core.layers[layer_id].render_order.len();
                                let operation       = LayerOperation::Draw;

                                core.layers[layer_id].render_order.push(RenderEntity::Tessellating(operation));

                                let entity          = LayerEntityRef { layer_id, entity_index };

                                // Create the canvas job
                                CanvasJob::Fill { path, color, entity, operation }
                            });

                            job_publisher.publish(job).await;
                        }
                    }

                    // Draw a line around the current path
                    Stroke => {
                        // Update the active path if the builder exists
                        if let Some(path_builder) = path_builder.take() {
                            current_path = Some(path_builder.build());
                        }

                        // unimplemented!() -- TODO
                    }

                    // Set the line width
                    LineWidth(width) => {
                        core.sync(|core| core.layers[self.current_layer].stroke_settings.line_width = width);
                    }

                    // Set the line width in pixels
                    LineWidthPixels(pixel_width) => {
                        // unimplemented!()
                    }

                    // Line join
                    LineJoin(join_type) => {
                        core.sync(|core| core.layers[self.current_layer].stroke_settings.join = join_type);
                    }

                    // The cap to use on lines
                    LineCap(cap_type) => {
                        core.sync(|core| core.layers[self.current_layer].stroke_settings.cap = cap_type);
                    }

                    // Resets the dash pattern to empty (which is a solid line)
                    NewDashPattern => {
                        core.sync(|core| core.layers[self.current_layer].stroke_settings.dash_pattern = vec![]);
                    }

                    // Adds a dash to the current dash pattern
                    DashLength(dash_length) => {
                        core.sync(|core| core.layers[self.current_layer].stroke_settings.dash_pattern.push(dash_length));
                    }

                    // Sets the offset for the dash pattern
                    DashOffset(offset) => {
                        core.sync(|core| core.layers[self.current_layer].stroke_settings.dash_offset = offset);
                    }

                    // Set the fill color
                    FillColor(color) => {
                        core.sync(|core| core.layers[self.current_layer].fill_color = Self::render_color(color));
                    }

                    // Set the line color
                    StrokeColor(color) => {
                        core.sync(|core| core.layers[self.current_layer].stroke_settings.stroke_color = Self::render_color(color));
                    }

                    // Set how future renderings are blended with one another
                    BlendMode(blend_mode) => {
                        //unimplemented!()
                    }

                    // Reset the transformation to the identity transformation
                    IdentityTransform => {
                        //unimplemented!()
                    }

                    // Sets a transformation such that:
                    // (0,0) is the center point of the canvas
                    // (0,height/2) is the top of the canvas
                    // Pixels are square
                    CanvasHeight(height) => {
                        //unimplemented!()
                    }

                    // Moves a particular region to the center of the canvas (coordinates are minx, miny, maxx, maxy)
                    CenterRegion((x1, y1), (x2, y2)) => {
                        //unimplemented!()
                    }

                    // Multiply a 2D transform into the canvas
                    MultiplyTransform(transform) => {
                        //unimplemented!()
                    }

                    // Unset the clipping path
                    Unclip => {
                        //unimplemented!()
                    }

                    // Clip to the currently set path
                    Clip => {
                        //unimplemented!()
                    }

                    // Stores the content of the clipping path from the current layer in a background buffer
                    Store => {
                        //unimplemented!()
                    }

                    // Restores what was stored in the background buffer. This should be done on the
                    // same layer that the Store operation was called upon.
                    //
                    // The buffer is left intact by this operation so it can be restored again in the future.
                    //
                    // (If the clipping path has changed since then, the restored image is clipped against the new path)
                    Restore => {
                        //unimplemented!()
                    }

                    // Releases the buffer created by the last 'Store' operation
                    //
                    // Restore will no longer be valid for the current layer
                    FreeStoredBuffer => {
                        //unimplemented!()
                    }

                    // Push the current state of the canvas (line settings, stored image, current path - all state)
                    PushState => {
                        //unimplemented!()
                    }

                    // Restore a state previously pushed
                    PopState => {
                        //unimplemented!()
                    }

                    // Clears the canvas entirely
                    ClearCanvas => {
                        //todo!("Stop any incoming tessellated data for this layer");
                        //todo!("Mark vertex buffers as freed");

                        core.sync(|core| {
                            core.layers         = vec![self.create_default_layer()];
                            self.current_layer  = 0;
                        });
                    }

                    // Selects a particular layer for drawing
                    // Layer 0 is selected initially. Layers are drawn in order starting from 0.
                    // Layer IDs don't have to be sequential.
                    Layer(layer_id) => {
                        let layer_id = layer_id as usize;

                        // Generate layers 
                        core.sync(|core| {
                            while layer_id <= core.layers.len() {
                                core.layers.push(self.create_default_layer());
                            }

                            self.current_layer = layer_id;
                        });
                    }

                    // Sets how a particular layer is blended with the underlying layer
                    LayerBlend(layer_id, blend_mode) => {
                        //unimplemented!()
                    }

                    // Clears the current layer
                    ClearLayer => {
                        //todo!("Stop any incoming tessellated data for this layer");
                        //todo!("Mark vertex buffers as freed");

                        core.sync(|core| core.layers[self.current_layer] = self.create_default_layer());
                    }
                }
            }

            // Wait for any pending jobs to make it to the processor
            job_publisher.when_empty().await;
        }
    }

    ///
    /// Starts processing a drawing, returning a future that completes once all of the tessellation operations
    /// have finished
    ///
    pub fn process_drawing<'a, DrawIter: 'a+Iterator<Item=canvas::Draw>>(&'a mut self, drawing: DrawIter) -> impl 'a+Future<Output=()> {
        // Create a copy of the core
        let core                    = Arc::clone(&self.core);
        let workers                 = self.workers.clone();

        // Send the jobs from the tessellator to the workers
        let mut publisher           = SinglePublisher::new(1);
        let job_results             = workers.into_iter()
            .map(|worker| {
                let jobs = publisher.subscribe();
                pipe(worker, jobs, |worker, item| {
                    async move {
                        worker.process_job(item)
                    }.boxed()
                })
            });
        let mut job_results         = futures::stream::select_all(job_results);

        // Start processing the drawing, and sending jobs to be tessellated
        let process_drawing         = self.tessellate(drawing, publisher);

        // Take the results and put them into the core
        let process_tessellations    = async move {
            // Iterate through the job results
            while let Some((entity, operation)) = job_results.next().await {
                // Store each result in the core
                core.sync(|core| core.store_job_result(entity, operation));
            }
        };

        // Combine the two futures for the end result
        futures::future::join(process_drawing, process_tessellations)
            .map(|_| ())
    }

    ///
    /// Returns a stream of render actions after applying a set of canvas drawing operations to this renderer
    ///
    pub fn draw<'a, DrawIter: 'a+Iterator<Item=canvas::Draw>>(&'a mut self, drawing: DrawIter) -> impl 'a+Stream<Item=render::RenderAction> {
        let core        = Arc::clone(&self.core);
        let processing  = self.process_drawing(drawing);

        RenderStream {
            core:               core,
            processing_future:  Some(processing.boxed_local()),
            layer_id:           0,
            render_index:       0,
            pending:            vec![
                render::RenderAction::Clear(render::Rgba8([0, 0, 0, 0]))
            ]
        }
    }
}

///
/// Stream of rendering actions resulting from a draw instruction
///
struct RenderStream<'a> {
    /// The core where the render instructions are read from
    core: Arc<Desync<RenderCore>>,

    /// The future that is processing new drawing instructions
    processing_future: Option<LocalBoxFuture<'a, ()>>,

    /// The current layer ID that we're processing
    layer_id: usize,

    /// The render entity within the layer that we're processing
    render_index: usize,

    /// Render actions waiting to be sent
    pending: Vec<render::RenderAction>
}

impl<'a> Stream for RenderStream<'a> {
    type Item = render::RenderAction;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<render::RenderAction>> { 
        // Return the next pending action if there is one
        if self.pending.len() > 0 {
            // Note that pending is a stack, so the items are returned in reverse order
            return Poll::Ready(self.pending.pop());
        }

        // Poll the tessellation process if it's still running
        if let Some(processing_future) = self.processing_future.as_mut() {
            // Poll the future and send over any vertex buffers that might be waiting
            if processing_future.poll_unpin(context) == Poll::Pending {
                // Still generating render buffers: scan the core to see if we can send any across
                let mut layer_id        = self.layer_id;
                let mut render_index    = self.render_index;

                let action = self.core.sync(|core| {
                    // Clip the layer ID, index
                    if core.layers.len() == 0 { return None; }
                    if layer_id >= core.layers.len() {
                        layer_id        = 0;
                        render_index    = 0;
                    }
                    if render_index > core.layers[layer_id].render_order.len() {
                        render_index = core.layers[layer_id].render_order.len();
                    }

                    // Set the initial layer ID and render index
                    let initial_layer_id        = layer_id;
                    let initial_render_index    = render_index;

                    // TODO: loop through the layer instructions

                    // No action
                    return None;
                });

                self.layer_id       = layer_id;
                self.render_index   = render_index;

                // TODO: can also send actual rendering instrucitons here, though we currently don't because we can't 
                // tell if a layer is 'finished' or not: we could send things out of order or rendering instructions 
                // that are later cleared

                // Actions are still pending
                if let Some(action) = action {
                    // Return the action we generated earlier
                    return Poll::Ready(Some(action));
                } else {
                    // Will generate the render actions once the draw commands have finished tessellating
                    return Poll::Pending;
                }
            } else {
                // Finished processing the rendering: can send the actual rendering commands to the hardware layer
                self.processing_future  = None;
                self.layer_id           = 0;
                self.render_index       = 0;
            }

        }

        // We've generated all the vertex buffers: generate the instructions to render them
        let mut layer_id        = self.layer_id;
        let mut render_index    = self.render_index;

        let result = self.core.sync(|core| {
            loop {
                if layer_id >= core.layers.len() {
                    // Reached the end of the layers
                    return vec![];
                }

                if render_index >= core.layers[layer_id].render_order.len() {
                    // Reached the end of the current layer
                    layer_id        += 1;
                    render_index    = 0;
                } else {
                    // layer_id, render_index is valid
                    break;
                }
            }

            // Action depends on the contents of the current render item
            use self::RenderEntity::*;
            match &core.layers[layer_id].render_order[render_index] {
                Tessellating(operation) => { 
                    // Being processed? (shouldn't happen)
                    panic!("Tessellation is not complete");
                },

                VertexBuffer(operation, buffers) => {
                    // Ask the core to send this buffer for processing
                    core.send_vertex_buffer(layer_id, render_index)
                },


                DrawIndexed(_op, vertex_buffer, index_buffer, num_items) => {
                    vec![render::RenderAction::DrawIndexedTriangles(*vertex_buffer, *index_buffer, *num_items)]
                }
            }
        });

        // Update the layer and render index to continue iterating
        self.layer_id       = layer_id;
        self.render_index   = render_index;

        // Add the result to the pending queue
        if result.len() > 0 {
            self.pending = result;
            return Poll::Ready(self.pending.pop());
        } else {
            // No further actions if the result was empty
            return Poll::Ready(None);
        }
    }
}