use super::stroke_settings::*;
use super::renderer_core::*;
use super::renderer_layer::*;
use super::renderer_worker::*;
use super::renderer_stream::*;

use flo_render as render;
use flo_canvas as canvas;
use flo_stream::*;

use ::desync::*;

use futures::prelude::*;
use num_cpus;
use lyon::path;
use lyon::math;

use std::ops::{Range};
use std::sync::*;
use std::mem;

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

    /// The viewport transformation
    viewport_transform: canvas::Transform2D,

    /// The inverse of the viewport transformation
    inverse_viewport_transform: canvas::Transform2D,

    /// The currently active transformation
    active_transform: canvas::Transform2D,

    /// The next ID to assign to an entity for tessellation
    next_entity_id: usize,

    /// The width and size of the window overall
    window_size: (f32, f32)
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
            workers.push(Arc::new(Desync::new(CanvasWorker::new())));
        }

        // Generate the final renderer
        CanvasRenderer {
            workers:                    workers,
            core:                       core,
            current_layer:              0,
            viewport_transform:         canvas::Transform2D::identity(),
            inverse_viewport_transform: canvas::Transform2D::identity(),
            active_transform:           canvas::Transform2D::identity(),
            next_entity_id:             0,
            window_size:                (1.0, 1.0),
        }
    }

    ///
    /// Sets the viewport used by this renderer
    ///
    pub fn set_viewport(&mut self, x: Range<f32>, y: Range<f32>, window_width: f32, window_height: f32) {
        // By default the x and y coordinates go from -1.0 to 1.0
        let width                       = x.end-x.start;
        let height                      = y.end-y.start;
        let scale_transform             = canvas::Transform2D::scale(2.0/width, 2.0/height);

        // Bottom-right corner is currently -width/2.0, -height/2.0 (as we scale around the center)
        let viewport_transform          = scale_transform * canvas::Transform2D::translate(-(width/2.0) + x.start, -(height/2.0) + y.start);
        let inverse_viewport_transform  = viewport_transform.invert().unwrap();

        self.viewport_transform         = viewport_transform;
        self.inverse_viewport_transform = inverse_viewport_transform;
        self.window_size                = (window_width, window_height);
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
                            let entity_id   = self.next_entity_id;

                            self.next_entity_id += 1;

                            let job         = core.sync(move |core| {
                                // Create the render entity in the tessellating state
                                let color               = core.layers[layer_id].fill_color;
                                let entity_index        = core.layers[layer_id].render_order.len();
                                let operation           = LayerOperation::Draw;

                                core.layers[layer_id].render_order.push(RenderEntity::Tessellating(operation, entity_id));

                                let entity          = LayerEntityRef { layer_id, entity_index, entity_id };

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
                        let canvas::Transform2D(transform)  = &self.active_transform;
                        let scale                           = (transform[0][0]*transform[0][0] + transform[1][1]*transform[1][1]).sqrt();
                        let width                           = pixel_width / scale;
                        
                        core.sync(|core| core.layers[self.current_layer].stroke_settings.line_width = width);
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
                        self.active_transform = canvas::Transform2D::identity();
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
                        self.active_transform = self.active_transform * transform;
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
                            // Create the new layers
                            let mut layers      = vec![self.create_default_layer()];

                            // Swap into the core
                            mem::swap(&mut core.layers, &mut layers);
                            self.current_layer  = 0;

                            // Free all the entities in all the layers
                            for layer in layers {
                                core.free_layer_entities(layer);
                            }
                        });
                        self.active_transform   = canvas::Transform2D::identity();
                    }

                    // Selects a particular layer for drawing
                    // Layer 0 is selected initially. Layers are drawn in order starting from 0.
                    // Layer IDs don't have to be sequential.
                    Layer(layer_id) => {
                        let layer_id = layer_id as usize;

                        // Generate layers 
                        core.sync(|core| {
                            while core.layers.len() <= layer_id  {
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
                        core.sync(|core| {
                            // Create a new layer
                            let mut layer = self.create_default_layer();

                            // Swap into the layer list to replace the old one
                            mem::swap(&mut core.layers[self.current_layer], &mut layer);

                            // Free the data for the current layer
                            core.free_layer_entities(layer);
                        });
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
        let core                = Arc::clone(&self.core);
        let viewport_matrix     = transform_to_matrix(&self.viewport_transform);
        let processing          = self.process_drawing(drawing);

        RenderStream::new(core, processing, vec![
            render::RenderAction::SetTransform(viewport_matrix),
            render::RenderAction::Clear(render::Rgba8([0, 0, 0, 0]))
        ])
    }
}
