use super::stroke_settings::*;
use super::renderer_core::*;
use super::renderer_layer::*;
use super::renderer_worker::*;
use super::renderer_stream::*;

use flo_render as render;
use flo_render::{RenderTargetId, TextureId, RenderTargetType};
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

    /// The transforms pushed to the stack when PushState was called
    transform_stack: Vec<canvas::Transform2D>,

    /// The next ID to assign to an entity for tessellation
    next_entity_id: usize,

    /// The width and size of the window overall
    window_size: (f32, f32),

    /// True if the MSAA rendering surface has been created
    created_render_surface: bool
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
            transform_stack:            vec![],
            next_entity_id:             0,
            window_size:                (1.0, 1.0),
            created_render_surface:     false
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

        if self.window_size != (window_width, window_height) {
            self.window_size            = (window_width, window_height);
            self.created_render_surface = false;
        }
    }

    ///
    /// Retrieves the active transform for the canvas (which is fully up to date after rendering)
    ///
    pub fn get_active_transform(&self) -> canvas::Transform2D {
        self.active_transform
    }

    ///
    /// Creates a new layer with the default properties
    ///
    fn create_default_layer(&self) -> Layer {
        Layer {
            render_order:       vec![RenderEntity::SetTransform(canvas::Transform2D::identity())],
            state:              LayerState {
                fill_color:         render::Rgba8([0, 0, 0, 255]),
                stroke_settings:    StrokeSettings::new(),
                current_matrix:     canvas::Transform2D::identity(),
                restore_point:      None
            },
            stored_states:      vec![]
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
                            let active_transform = &self.active_transform;

                            self.next_entity_id += 1;

                            let job         = core.sync(move |core| {
                                // Update the transformation matrix
                                core.layers[layer_id].update_transform(active_transform);

                                // Create the render entity in the tessellating state
                                let color               = core.layers[layer_id].state.fill_color;
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

                        // Publish the job to the tessellators
                        if let Some(path) = &current_path {
                            let path        = path.clone();
                            let layer_id    = self.current_layer;
                            let entity_id   = self.next_entity_id;
                            let active_transform = &self.active_transform;

                            self.next_entity_id += 1;

                            let job         = core.sync(move |core| {
                                // Update the transformation matrix
                                core.layers[layer_id].update_transform(active_transform);

                                // Create the render entity in the tessellating state
                                let stroke_options      = core.layers[layer_id].state.stroke_settings.clone();
                                let entity_index        = core.layers[layer_id].render_order.len();
                                let operation           = LayerOperation::Draw;

                                core.layers[layer_id].render_order.push(RenderEntity::Tessellating(operation, entity_id));

                                let entity          = LayerEntityRef { layer_id, entity_index, entity_id };

                                // Create the canvas job
                                CanvasJob::Stroke { path, stroke_options, entity, operation }
                            });

                            job_publisher.publish(job).await;
                        }
                    }

                    // Set the line width
                    LineWidth(width) => {
                        core.sync(|core| core.layers[self.current_layer].state.stroke_settings.line_width = width);
                    }

                    // Set the line width in pixels
                    LineWidthPixels(pixel_width) => {
                        let canvas::Transform2D(transform)  = &self.active_transform;
                        let scale                           = (transform[0][0]*transform[0][0] + transform[1][0]*transform[1][0]).sqrt();
                        let width                           = pixel_width / scale;

                        core.sync(|core| core.layers[self.current_layer].state.stroke_settings.line_width = width);
                    }

                    // Line join
                    LineJoin(join_type) => {
                        core.sync(|core| core.layers[self.current_layer].state.stroke_settings.join = join_type);
                    }

                    // The cap to use on lines
                    LineCap(cap_type) => {
                        core.sync(|core| core.layers[self.current_layer].state.stroke_settings.cap = cap_type);
                    }

                    // Resets the dash pattern to empty (which is a solid line)
                    NewDashPattern => {
                        core.sync(|core| core.layers[self.current_layer].state.stroke_settings.dash_pattern = vec![]);
                    }

                    // Adds a dash to the current dash pattern
                    DashLength(dash_length) => {
                        core.sync(|core| core.layers[self.current_layer].state.stroke_settings.dash_pattern.push(dash_length));
                    }

                    // Sets the offset for the dash pattern
                    DashOffset(offset) => {
                        core.sync(|core| core.layers[self.current_layer].state.stroke_settings.dash_offset = offset);
                    }

                    // Set the fill color
                    FillColor(color) => {
                        core.sync(|core| core.layers[self.current_layer].state.fill_color = Self::render_color(color));
                    }

                    // Set the line color
                    StrokeColor(color) => {
                        core.sync(|core| core.layers[self.current_layer].state.stroke_settings.stroke_color = Self::render_color(color));
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
                        // Work out the scale to use for this widget
                        let height              = f32::max(1.0, height);
                        let scale               = self.window_size.1 / height;
                        let scale               = canvas::Transform2D::scale(scale, scale);

                        // The viewport transform makes (0,0) the lower-left of the canvas: move it to the middle
                        let translate           = canvas::Transform2D::translate(self.window_size.0/2.0, self.window_size.1/2.0);
                        let transform           = translate * scale;

                        // Set as the active transform
                        self.active_transform   = transform;
                    }

                    // Moves a particular region to the center of the canvas (coordinates are minx, miny, maxx, maxy)
                    CenterRegion((x1, y1), (x2, y2)) => {
                        // Work out the scale factor
                        let region_width        = f32::max(0.0, x2-x1);
                        let region_height       = f32::max(0.0, y2-y1);

                        let scale_x             = self.window_size.0 / region_width;
                        let scale_y             = self.window_size.1 / region_height;
                        let scale               = f32::min(scale_x, scale_y);

                        let scale               = canvas::Transform2D::scale(scale, scale);

                        // Move the center point to the middle of the canvas
                        let center_x            = (x1+x2)/2.0;
                        let center_y            = (y1+y2)/2.0;
                        let left_x              = center_x - self.window_size.0/2.0;
                        let left_y              = center_y - self.window_size.1/2.0;

                        let translate           = canvas::Transform2D::translate(left_x, left_y);

                        // Combine to set the active transform
                        let transform           = translate * scale;
                        self.active_transform   = transform;
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
                        // TODO: this does not support the clipping behaviour (it stores/restores the whole layer)
                        // (We currently aren't using the clipping behaviour for anything so it might be easier to just
                        // remove that capability from the documentation?)
                        core.sync(|core| core.layers[self.current_layer].state.restore_point = Some(core.layers[self.current_layer].render_order.len()));
                    }

                    // Restores what was stored in the background buffer. This should be done on the
                    // same layer that the Store operation was called upon.
                    //
                    // The buffer is left intact by this operation so it can be restored again in the future.
                    //
                    // (If the clipping path has changed since then, the restored image is clipped against the new path)
                    Restore => {
                        // Roll back the layer to the restore point
                        core.sync(|core| {
                            if let Some(restore_point) = core.layers[self.current_layer].state.restore_point {
                                // Remove entries from the layer until we reach the restore point
                                while core.layers[self.current_layer].render_order.len() > restore_point {
                                    let removed_entity = core.layers[self.current_layer].render_order.pop();
                                    removed_entity.map(|removed| core.free_entity(removed));
                                }
                            }
                        })
                    }

                    // Releases the buffer created by the last 'Store' operation
                    //
                    // Restore will no longer be valid for the current layer
                    FreeStoredBuffer => {
                        core.sync(|core| core.layers[self.current_layer].state.restore_point = None);
                    }

                    // Push the current state of the canvas (line settings, stored image, current path - all state)
                    PushState => {
                        self.transform_stack.push(self.active_transform);

                        core.sync(|core| {
                            for layer in core.layers.iter_mut() {
                                layer.push_state();
                            }
                        })
                    }

                    // Restore a state previously pushed
                    PopState => {
                        self.transform_stack.pop()
                            .map(|transform| self.active_transform = transform);

                        core.sync(|core| {
                            for layer in core.layers.iter_mut() {
                                layer.pop_state();
                            }
                        })
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
            // Read job results from the workers until everything is done
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
        // Set up the initial set of rendering actions
        let viewport_transform  = self.viewport_transform;
        let viewport_matrix     = transform_to_matrix(&self.viewport_transform);
        let mut initialise      = vec![
            render::RenderAction::SetTransform(viewport_matrix),
            render::RenderAction::Clear(render::Rgba8([0, 0, 0, 0])),
            render::RenderAction::SelectRenderTarget(RenderTargetId(0))
        ];

        if !self.created_render_surface {
            // If the MSAA render surface is missing, create it (it's always render target 0, texture 0)
            initialise.push(render::RenderAction::CreateRenderTarget(RenderTargetId(0), TextureId(0), 
                self.window_size.0 as usize,
                self.window_size.1 as usize,
                RenderTargetType::Multisampled));

            self.created_render_surface = true;
        }

        // When finished, render the MSAA buffer to the main framebuffer
        let finalize            = vec![
            render::RenderAction::DrawFrameBuffer(RenderTargetId(0), 0, 0),
            render::RenderAction::RenderToFrameBuffer
        ];

        // Start processing the drawing instructions
        let core                = Arc::clone(&self.core);
        let processing          = self.process_drawing(drawing);

        // Return a stream of results from processing the drawing
        RenderStream::new(core, processing, viewport_transform, initialise, finalize)
    }
}
