use super::layer_state::*;
use super::render_entity::*;
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
use lyon::tessellation::{FillRule};

use std::collections::{HashMap};
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
    current_layer: LayerHandle,

    /// The viewport transformation (this makes for rectangular pixels with the bottom of the window at 0, -1 and the top at 0, 1)
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

    /// The scale factor of the window
    window_scale: f32,

    /// The origin of the viewport
    viewport_origin: (f32, f32),

    /// The width and size of the viewport we're rendering to
    viewport_size: (f32, f32),

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
            free_layers:            vec![],
            layer_definitions:      vec![],
            sprites:                HashMap::new(),
            unused_vertex_buffer:   0,
            free_vertex_buffers:    vec![]
        };
        let core = Arc::new(Desync::new(core));

        // Create the initial layer
        let initial_layer = Self::create_default_layer();
        let initial_layer = core.sync(move |core| {
            let layer0 = core.allocate_layer_handle(initial_layer);
            core.layers.push(layer0);
            layer0
        });

        // Create one worker per cpu
        let num_workers = num_cpus::get().max(2);
        let mut workers = Vec::with_capacity(num_workers);

        for _ in 0..num_workers {
            workers.push(Arc::new(Desync::new(CanvasWorker::new())));
        }

        // Generate the final renderer
        CanvasRenderer {
            workers:                    workers,
            core:                       core,
            current_layer:              initial_layer,
            viewport_transform:         canvas::Transform2D::identity(),
            inverse_viewport_transform: canvas::Transform2D::identity(),
            active_transform:           canvas::Transform2D::identity(),
            transform_stack:            vec![],
            next_entity_id:             0,
            window_size:                (1.0, 1.0),
            window_scale:               1.0,
            viewport_origin:            (0.0, 0.0),
            viewport_size:              (1.0, 1.0),
            created_render_surface:     false
        }
    }

    ///
    /// Sets the viewport used by this renderer
    ///
    /// The window width and height is the overall size of the canvas (which can be considered to have 
    /// coordinates from 0,0 to window_width, window_height). The viewport, given by x and y here, is the
    /// region of the window that will actually be rendered.
    ///
    /// The viewport and window coordinates are all in pixels. The scale used when generating transformations
    /// (so with a scale of 2, a CanvasHeight request of 1080 will act as a height 2160 in the viewport).
    ///
    pub fn set_viewport(&mut self, x: Range<f32>, y: Range<f32>, window_width: f32, window_height: f32, scale: f32) {
        // By default the x and y coordinates go from -1.0 to 1.0 and represent the viewport coordinates

        // Width and height of the viewport
        let width                       = x.end-x.start;
        let height                      = y.end-y.start;

        // Widths/heights of 0.0 will cause issues with calculating ratios and scales
        let window_width                = if window_width == 0.0 { 1.0 } else { window_width };
        let window_height               = if window_height == 0.0 { 1.0 } else { window_height };
        let width                       = if width == 0.0 { 1.0 } else { width };
        let height                      = if height == 0.0 { 1.0 } else { height };

        // Create a scale to make the viewport have square pixels (the viewport is the shape of our render surface)
        let viewport_ratio              = height / width;
        let square_pixels               = canvas::Transform2D::scale(viewport_ratio, 1.0);

        // Viewport is scaled and translated relative to the window size
        let pixel_size                  = 2.0 / window_height;
        let window_scale                = window_height / height;

        // Want to move the center of the display to the center of the viewport
        let window_mid_x                = window_width/2.0;
        let window_mid_y                = window_height/2.0;
        let viewport_mid_x              = (x.start + x.end) / 2.0;
        let viewport_mid_y              = (y.start + y.end) / 2.0;
        let translate_x                 = (window_mid_x-viewport_mid_x) * pixel_size;
        let translate_y                 = (window_mid_y-viewport_mid_y) * pixel_size;

        // Create a viewport transform such that the top of the window is at (0,1) and the bottom is at (0,-1)
        let viewport_transform          = square_pixels * canvas::Transform2D::scale(window_scale, window_scale) * canvas::Transform2D::translate(translate_x, translate_y);
        let inverse_viewport_transform  = viewport_transform.invert().unwrap();

        // Store the size of the window
        self.viewport_transform         = viewport_transform;
        self.inverse_viewport_transform = inverse_viewport_transform;

        if self.window_size != (window_width, window_height) {
            self.window_size            = (window_width, window_height);
            self.created_render_surface = false;
        }

        let viewport_width              = x.end-x.start;
        let viewport_height             = y.end-y.start;

        self.viewport_origin            = (x.start, y.start);
        self.window_scale               = scale;

        if self.viewport_size != (viewport_width, viewport_height) {
            self.viewport_size          = (viewport_width, viewport_height);
            self.created_render_surface = false;
        }
    }

    ///
    /// Returns the coordinates of the viewport, as x and y ranges
    ///
    pub fn get_viewport(&self) -> (Range<f32>, Range<f32>) {
        let x_range = self.viewport_origin.0..(self.viewport_origin.0 + self.viewport_size.0);
        let y_range = self.viewport_origin.1..(self.viewport_origin.1 + self.viewport_size.1);

        (x_range, y_range)
    }

    ///
    /// Retrieves the active transform for the canvas (which is fully up to date after rendering)
    ///
    pub fn get_active_transform(&self) -> canvas::Transform2D {
        self.active_transform
    }

    ///
    /// Retrieves a transformation that maps a point from canvas coordinates to viewport coordinates
    ///
    pub fn get_viewport_transform(&self) -> canvas::Transform2D {
        let to_normalized_coordinates   = self.get_active_transform();
        let scale_x                     = self.window_size.0/2.0;
        let scale_y                     = self.window_size.1/2.0;

        canvas::Transform2D::translate(self.viewport_origin.0, self.viewport_origin.1)
            * canvas::Transform2D::scale(scale_y, scale_y)
            * canvas::Transform2D::translate(scale_x/scale_y, 1.0) 
            * to_normalized_coordinates 
    }

    ///
    /// Retrieves a transformation that maps a point from canvas coordinates to window coordinates
    ///
    pub fn get_window_transform(&self) -> canvas::Transform2D {
        let to_normalized_coordinates   = self.get_active_transform();
        let scale_x                     = self.window_size.0/2.0;
        let scale_y                     = self.window_size.1/2.0;

        canvas::Transform2D::scale(scale_y, scale_y)
            * canvas::Transform2D::translate(scale_x/scale_y, 1.0) 
            * to_normalized_coordinates 
    }

    ///
    /// Creates a new layer with the default properties
    ///
    fn create_default_layer() -> Layer {
        Layer {
            render_order:       vec![RenderEntity::SetTransform(canvas::Transform2D::identity())],
            state:              LayerState {
                is_sprite:          false,
                fill_color:         render::Rgba8([0, 0, 0, 255]),
                winding_rule:       FillRule::NonZero,
                stroke_settings:    StrokeSettings::new(),
                current_matrix:     canvas::Transform2D::identity(),
                sprite_matrix:      canvas::Transform2D::identity(),
                blend_mode:         canvas::BlendMode::SourceOver,
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
    fn tessellate<'a, DrawIter: 'a+Iterator<Item=canvas::Draw>>(&'a mut self, drawing: DrawIter, job_publisher: SinglePublisher<Vec<CanvasJob>>) -> impl 'a+Future<Output=()> {
        async move {
            let core                = Arc::clone(&self.core);
            let mut job_publisher   = job_publisher;
            let mut pending_jobs    = vec![];
            let batch_size          = 20;

            // The current path that is being built up
            let mut path_builder    = None;

            // The last path that was generated
            let mut current_path    = None;

            // Create the default layer if one doesn't already exist
            core.sync(|core| {
                if core.layers.len() == 0 {
                    let layer0          = Self::create_default_layer();
                    let layer0          = core.allocate_layer_handle(layer0);
                    core.layers         = vec![layer0];
                    self.current_layer  = layer0;
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
                            let path                = path.clone();
                            let layer_id            = self.current_layer;
                            let entity_id           = self.next_entity_id;
                            let active_transform    = &self.active_transform;

                            self.next_entity_id += 1;

                            let job         = core.sync(move |core| {
                                let layer               = core.layer(layer_id);

                                // Update the transformation matrix
                                layer.update_transform(active_transform);

                                // Create the render entity in the tessellating state
                                let color               = layer.state.fill_color;
                                let fill_rule           = layer.state.winding_rule;
                                let entity_index        = layer.render_order.len();

                                // When drawing to the erase layer (DesintationOut blend mode), all colour components are alpha components
                                let color               = if layer.state.blend_mode == canvas::BlendMode::DestinationOut { render::Rgba8([color.0[3], color.0[3], color.0[3], color.0[3]]) } else { color };

                                layer.render_order.push(RenderEntity::Tessellating(entity_id));

                                let entity          = LayerEntityRef { layer_id, entity_index, entity_id };

                                // Create the canvas job
                                CanvasJob::Fill { path, fill_rule, color, entity }
                            });

                            pending_jobs.push(job);
                            if pending_jobs.len() >= batch_size {
                                job_publisher.publish(pending_jobs).await;
                                pending_jobs = vec![];
                            }
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
                                let layer               = core.layer(layer_id);

                                // Update the transformation matrix
                                layer.update_transform(active_transform);

                                // Create the render entity in the tessellating state
                                let mut stroke_options  = layer.state.stroke_settings.clone();
                                let entity_index        = layer.render_order.len();


                                // When drawing to the erase layer (DesintationOut blend mode), all colour components are alpha components
                                let color                   = stroke_options.stroke_color;
                                stroke_options.stroke_color = if layer.state.blend_mode == canvas::BlendMode::DestinationOut { render::Rgba8([color.0[3], color.0[3], color.0[3], color.0[3]]) } else { color };

                                layer.render_order.push(RenderEntity::Tessellating(entity_id));

                                let entity          = LayerEntityRef { layer_id, entity_index, entity_id };

                                // Create the canvas job
                                CanvasJob::Stroke { path, stroke_options, entity }
                            });

                            pending_jobs.push(job);
                            if pending_jobs.len() >= batch_size {
                                job_publisher.publish(pending_jobs).await;
                                pending_jobs = vec![];
                            }
                        }
                    }

                    // Set the line width
                    LineWidth(width) => {
                        core.sync(|core| core.layer(self.current_layer).state.stroke_settings.line_width = width);
                    }

                    // Set the line width in pixels
                    LineWidthPixels(pixel_width) => {
                        // TODO: if the window width changes we won't re-tessellate the lines affected by this line width
                        let canvas::Transform2D(transform)  = &self.active_transform;
                        let pixel_size                      = 2.0/self.window_size.1 * self.window_scale;
                        let pixel_width                     = pixel_width * pixel_size;
                        let scale                           = (transform[0][0]*transform[0][0] + transform[1][0]*transform[1][0]).sqrt();
                        let width                           = pixel_width / scale;

                        core.sync(|core| core.layer(self.current_layer).state.stroke_settings.line_width = width);
                    }

                    // Line join
                    LineJoin(join_type) => {
                        core.sync(|core| core.layer(self.current_layer).state.stroke_settings.join = join_type);
                    }

                    // The cap to use on lines
                    LineCap(cap_type) => {
                        core.sync(|core| core.layer(self.current_layer).state.stroke_settings.cap = cap_type);
                    }

                    // The winding rule to use when filling areas
                    WindingRule(canvas::WindingRule::EvenOdd) => {
                        core.sync(|core| core.layer(self.current_layer).state.winding_rule = FillRule::EvenOdd);
                    }
                    WindingRule(canvas::WindingRule::NonZero) => {
                        core.sync(|core| core.layer(self.current_layer).state.winding_rule = FillRule::NonZero);
                    }

                    // Resets the dash pattern to empty (which is a solid line)
                    NewDashPattern => {
                        core.sync(|core| core.layer(self.current_layer).state.stroke_settings.dash_pattern = vec![]);
                    }

                    // Adds a dash to the current dash pattern
                    DashLength(dash_length) => {
                        core.sync(|core| core.layer(self.current_layer).state.stroke_settings.dash_pattern.push(dash_length));
                    }

                    // Sets the offset for the dash pattern
                    DashOffset(offset) => {
                        core.sync(|core| core.layer(self.current_layer).state.stroke_settings.dash_offset = offset);
                    }

                    // Set the fill color
                    FillColor(color) => {
                        core.sync(|core| core.layer(self.current_layer).state.fill_color = Self::render_color(color));
                    }

                    // Set the line color
                    StrokeColor(color) => {
                        core.sync(|core| core.layer(self.current_layer).state.stroke_settings.stroke_color = Self::render_color(color));
                    }

                    // Set how future renderings are blended with one another
                    BlendMode(blend_mode) => {
                        core.sync(|core| {
                            use canvas::BlendMode::*;
                            core.layer(self.current_layer).state.blend_mode = blend_mode;

                            let blend_mode = match blend_mode {
                                SourceOver      => render::BlendMode::DestinationOver,
                                DestinationOver => render::BlendMode::SourceOver,
                                DestinationOut  => render::BlendMode::DestinationOut,

                                // TODO: these blend modes aren't supported yet
                                SourceIn        |
                                SourceOut       |
                                DestinationIn   |
                                SourceAtop      |
                                DestinationAtop |

                                Multiply        |
                                Screen          |
                                Darken          |
                                Lighten         => render::BlendMode::DestinationOver
                            };

                            core.layer(self.current_layer).render_order.push(RenderEntity::SetBlendMode(blend_mode));
                        });
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
                        // Window height is set at 2.0 by the viewport transform
                        let window_height       = 2.0;

                        // Work out the scale to use for this widget
                        let height              = f32::max(1.0, height);
                        let scale               = window_height / height;
                        let scale               = canvas::Transform2D::scale(scale, scale);

                        // (0, 0) is already the center of the window
                        let transform           = scale;

                        // Set as the active transform
                        self.active_transform   = transform;
                    }

                    // Moves a particular region to the center of the canvas (coordinates are minx, miny, maxx, maxy)
                    CenterRegion((x1, y1), (x2, y2)) => {
                        // Get the center point in viewport coordinates
                        let center_x                = 0.0;
                        let center_y                = 0.0;

                        // Find the current center point
                        let current_transform       = self.active_transform.clone();
                        let inverse_transform       = current_transform.invert().unwrap();

                        let (center_x, center_y)    = inverse_transform.transform_point(center_x, center_y);

                        // Translate the center point onto the center of the region
                        let (new_x, new_y)          = ((x1+x2)/2.0, (y1+y2)/2.0);
                        let translation             = canvas::Transform2D::translate(-(new_x - center_x), -(new_y - center_y));

                        self.active_transform       = self.active_transform * translation;
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
                        core.sync(|core| core.layer(self.current_layer).state.restore_point = Some(core.layer(self.current_layer).render_order.len()));
                    }

                    // Restores what was stored in the background buffer. This should be done on the
                    // same layer that the Store operation was called upon.
                    //
                    // The buffer is left intact by this operation so it can be restored again in the future.
                    //
                    // (If the clipping path has changed since then, the restored image is clipped against the new path)
                    Restore => {
                        // Roll back the layer to the restore point
                        // TODO: need to reset the blend mode
                        core.sync(|core| {
                            if let Some(restore_point) = core.layer(self.current_layer).state.restore_point {
                                let mut layer = core.layer(self.current_layer);

                                // Remove entries from the layer until we reach the restore point
                                while layer.render_order.len() > restore_point {
                                    let removed_entity = layer.render_order.pop();
                                    removed_entity.map(|removed| core.free_entity(removed));

                                    // Reborrow the layer after removal
                                    layer = core.layer(self.current_layer);
                                }
                            }
                        })
                    }

                    // Releases the buffer created by the last 'Store' operation
                    //
                    // Restore will no longer be valid for the current layer
                    FreeStoredBuffer => {
                        core.sync(|core| core.layer(self.current_layer).state.restore_point = None);
                    }

                    // Push the current state of the canvas (line settings, stored image, current path - all state)
                    PushState => {
                        self.transform_stack.push(self.active_transform);

                        core.sync(|core| {
                            for layer_id in core.layers.clone() {
                                core.layer(layer_id).push_state();
                            }
                        })
                    }

                    // Restore a state previously pushed
                    PopState => {
                        self.transform_stack.pop()
                            .map(|transform| self.active_transform = transform);

                        core.sync(|core| {
                            for layer_id in core.layers.clone() {
                                // The 'current matrix' is the matrix that's currently applied to the layer: it doesn't change when we pop the state
                                let layer_matrix = core.layer(layer_id).state.current_matrix;
                                core.layer(layer_id).pop_state();
                                core.layer(layer_id).state.current_matrix = layer_matrix;
                            }
                        })
                    }

                    // Clears the canvas entirely
                    ClearCanvas => {
                        //todo!("Stop any incoming tessellated data for this layer");
                        //todo!("Mark vertex buffers as freed");
                        core.sync(|core| {
                            // Release the existing layers
                            let mut old_layers = vec![];
                            mem::swap(&mut core.layers, &mut old_layers);

                            for layer_id in old_layers {
                                let layer = core.release_layer_handle(layer_id);
                                core.free_layer_entities(layer);
                            }

                            // Create a new default layer
                            let layer0 = Self::create_default_layer();
                            let layer0 = core.allocate_layer_handle(layer0);
                            core.layers.push(layer0);

                            self.current_layer = layer0;
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
                                let new_layer = Self::create_default_layer();
                                let new_layer = core.allocate_layer_handle(new_layer);
                                core.layers.push(new_layer);
                            }

                            self.current_layer = core.layers[layer_id];
                        });
                    }

                    // Sets how a particular layer is blended with the underlying layer
                    LayerBlend(_layer_id, _blend_mode) => {
                        // TODO: this needs some more work: for some blending modes we probably need to render the layer off-screen
                        // and the current 'reverse order' drawing makes drawing it in the right order tricky

                        //unimplemented!()
                    }

                    // Clears the current layer
                    ClearLayer | ClearSprite => {
                        core.sync(|core| {
                            // Create a new layer
                            let mut layer               = Self::create_default_layer();

                            // Sprite layers act as if their transform is already set
                            if core.layer(self.current_layer).state.is_sprite {
                                layer.state.is_sprite       = true;
                                layer.state.current_matrix  = self.active_transform;
                            }

                            // Swap into the layer list to replace the old one
                            mem::swap(core.layer(self.current_layer), &mut layer);

                            // Free the data for the current layer
                            core.free_layer_entities(layer);
                        });
                    },

                    // Selects a particular sprite for drawing
                    Sprite(sprite_id) => { 
                        core.sync(|core| {
                            if let Some(sprite_handle) = core.sprites.get(&sprite_id) {
                                // Use the existing sprite layer if one exists
                                self.current_layer = *sprite_handle;
                            } else {
                                // Create a new sprite layer
                                let mut sprite_layer            = Self::create_default_layer();
                                sprite_layer.state.is_sprite    = true;

                                // Associate it with the sprite ID
                                let sprite_layer                = core.allocate_layer_handle(sprite_layer);
                                core.sprites.insert(sprite_id, sprite_layer);

                                // Choose the layer as the current sprite layer
                                self.current_layer              = sprite_layer;
                            }

                            // Set the sprite matrix to be 'unchanged' from the active transform
                            let layer                   = core.layer(self.current_layer);
                            layer.state.current_matrix  = self.active_transform;
                        })
                    },

                    // Adds a sprite transform to the current list of transformations to apply
                    SpriteTransform(transform) => {
                        core.sync(|core| {
                            core.layer(self.current_layer).state.apply_sprite_transform(transform)
                        })
                    },

                    // Renders a sprite with a set of transformations
                    DrawSprite(sprite_id) => { 
                        core.sync(|core| {
                            let layer           = core.layer(self.current_layer);
                            let sprite_matrix   = layer.state.sprite_matrix;

                            // Update the transformation matrix
                            layer.update_transform(&self.active_transform);

                            // Render the sprite
                            layer.render_order.push(RenderEntity::RenderSprite(sprite_id, sprite_matrix))
                        })
                    },
                }
            }

            if pending_jobs.len() > 0 {
                job_publisher.publish(pending_jobs).await;
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
        let mut publisher           = SinglePublisher::new(2);
        let job_results             = workers.into_iter()
            .map(|worker| {
                let jobs = publisher.subscribe();
                pipe(worker, jobs, |worker, items: Vec<CanvasJob>| {
                    async move {
                        items.into_iter()
                            .map(|item| worker.process_job(item))
                            .collect::<Vec<_>>()
                    }.boxed()
                })
            });
        let mut job_results         = futures::stream::select_all(job_results);

        // Start processing the drawing, and sending jobs to be tessellated
        let process_drawing         = self.tessellate(drawing, publisher);

        // Take the results and put them into the core
        let process_tessellations    = async move {
            // Read job results from the workers until everything is done
            while let Some(result_list) = job_results.next().await {
                for (entity, operation) in result_list {
                    // Store each result in the core
                    core.sync(|core| core.store_job_result(entity, operation));
                }
            }
        };

        // Combine the two futures for the end result
        futures::future::join(process_drawing, process_tessellations)
            .map(|_| ())
    }

    ///
    /// Returns a stream of render actions after applying a set of canvas drawing operations to this renderer
    ///
    pub fn draw<'a, DrawIter: 'a+Send+Iterator<Item=canvas::Draw>>(&'a mut self, drawing: DrawIter) -> impl 'a+Send+Stream<Item=render::RenderAction> {
        // Set up the initial set of rendering actions
        let viewport_transform  = self.viewport_transform;
        let viewport_matrix     = transform_to_matrix(&self.viewport_transform);
        let mut initialise      = vec![
            render::RenderAction::SetTransform(viewport_matrix),
            render::RenderAction::Clear(render::Rgba8([0, 0, 0, 0])),
            render::RenderAction::BlendMode(render::BlendMode::DestinationOver),
            render::RenderAction::SelectRenderTarget(RenderTargetId(0)),
        ];

        if !self.created_render_surface {
            // If the MSAA render surface is missing, create it (it's always render target 0, texture 0)
            initialise.push(render::RenderAction::CreateRenderTarget(RenderTargetId(0), TextureId(0), 
                self.viewport_size.0 as usize,
                self.viewport_size.1 as usize,
                RenderTargetType::Multisampled));

            // Also create the 'eraser' render surface (render target 1, texture 1)
            initialise.push(render::RenderAction::CreateRenderTarget(RenderTargetId(1), TextureId(1),
                self.viewport_size.0 as usize,
                self.viewport_size.1 as usize,
                RenderTargetType::MonochromeMultisampledTexture));

            self.created_render_surface = true;
        }

        // When finished, render the MSAA buffer to the main framebuffer
        let finalize            = vec![
            render::RenderAction::DrawFrameBuffer(RenderTargetId(0), 0, 0),
            render::RenderAction::BlendMode(render::BlendMode::SourceOver),
            render::RenderAction::RenderToFrameBuffer
        ];

        // Start processing the drawing instructions
        let core                = Arc::clone(&self.core);
        let processing          = self.process_drawing(drawing);

        // Return a stream of results from processing the drawing
        RenderStream::new(core, processing, viewport_transform, initialise, finalize)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use flo_canvas::*;
    use futures::executor;

    #[test]
    pub fn active_transform_after_setting_canvas_height() {
        let mut renderer = CanvasRenderer::new();

        executor::block_on(async move {
            // Set the canvas height
            renderer.set_viewport(0.0..1024.0, 0.0..768.0, 1024.0, 768.0, 1.0);
            renderer.draw(vec![Draw::ClearCanvas, Draw::CanvasHeight(1000.0)].into_iter()).collect::<Vec<_>>().await;

            // Fetch the viewport transform
            let active_transform = renderer.get_active_transform();

            // The point 0, 500 should be at the top-middle of the viewport (height of 1000)
            let (x, y) = active_transform.transform_point(0.0, 500.0);
            assert!((x-0.0).abs() < 0.01);
            assert!((y-1.0).abs() < 0.01);

            // The point 500, 0 should be at the right of the viewport (height of 1000). Dimensions are in terms of the window height.
            let (x, y) = active_transform.transform_point(500.0, 0.0);
            assert!((y-0.0).abs() < 0.01);
            assert!((x-1.0).abs() < 0.01);
        });
    }

    #[test]
    pub fn active_transform_after_setting_canvas_height_in_big_window() {
        let mut renderer = CanvasRenderer::new();

        executor::block_on(async move {
            // Set the canvas height, viewport is half the window
            renderer.set_viewport(0.0..1024.0, 0.0..768.0, 2048.0, 1536.0, 1.0);
            renderer.draw(vec![Draw::ClearCanvas, Draw::CanvasHeight(1000.0)].into_iter()).collect::<Vec<_>>().await;

            // Fetch the viewport transform
            let active_transform = renderer.get_active_transform();

            // The point 0, 500 should be at the top-middle of the viewport (height of 1000)
            let (x, y) = active_transform.transform_point(0.0, 500.0);
            assert!((x-0.0).abs() < 0.01);
            assert!((y-1.0).abs() < 0.01);

            // The point 500, 0 should be at the right of the viewport (height of 1000). Dimensions are in terms of the window height.
            let (x, y) = active_transform.transform_point(500.0, 0.0);
            assert!((y-0.0).abs() < 0.01);
            assert!((x-1.0).abs() < 0.01);
        });
    }

    #[test]
    pub fn viewport_transform_after_setting_canvas_height() {
        let mut renderer = CanvasRenderer::new();

        executor::block_on(async move {
            // Set the canvas height
            renderer.set_viewport(0.0..1024.0, 0.0..768.0, 1024.0, 768.0, 1.0);
            renderer.draw(vec![Draw::ClearCanvas, Draw::CanvasHeight(1000.0)].into_iter()).collect::<Vec<_>>().await;

            // Fetch the viewport transform
            let viewport_transform = renderer.get_viewport_transform();

            // The point 0, 500 should be at the top-middle of the viewport (height of 1000)
            let (x, y) = viewport_transform.transform_point(0.0, 500.0);
            assert!((x-512.0).abs() < 0.01);
            assert!((y-768.0).abs() < 0.01);

            // The point 500, 0 should be at the right of the viewport (height of 1000). Pixels are square
            let (x, y) = viewport_transform.transform_point(500.0, 0.0);
            assert!((y-384.0).abs() < 0.01);
            assert!((x-896.0).abs() < 0.01);
        });
    }

    #[test]
    pub fn viewport_transform_after_setting_canvas_height_in_big_window() {
        let mut renderer = CanvasRenderer::new();

        executor::block_on(async move {
            // Set the canvas height
            renderer.set_viewport(0.0..1024.0, 0.0..768.0, 2048.0, 1536.0, 1.0);
            renderer.draw(vec![Draw::ClearCanvas, Draw::CanvasHeight(1000.0)].into_iter()).collect::<Vec<_>>().await;

            // Fetch the viewport transform
            let viewport_transform = renderer.get_viewport_transform();

            // The point 0, 500 should be at the top-middle of the viewport (height of 1000)
            let (x, y) = viewport_transform.transform_point(0.0, 500.0);
            assert!((x-1024.0).abs() < 0.01);
            assert!((y-1536.0).abs() < 0.01);

            // The point 500, 0 should be at the right of the viewport (height of 1000). Pixels are square
            let (x, y) = viewport_transform.transform_point(500.0, 0.0);
            assert!((y-768.0).abs() < 0.01);
            assert!((x-1792.0).abs() < 0.01);
        });
    }

    #[test]
    pub fn viewport_transform_after_setting_canvas_height_in_big_window_with_scroll() {
        let mut renderer = CanvasRenderer::new();

        executor::block_on(async move {
            // Set the canvas height
            renderer.set_viewport(512.0..1536.0, 512.0..1280.0, 2048.0, 1536.0, 1.0);
            renderer.draw(vec![Draw::ClearCanvas, Draw::CanvasHeight(1000.0)].into_iter()).collect::<Vec<_>>().await;

            // Fetch the viewport transform
            let viewport_transform = renderer.get_viewport_transform();

            // The point 0, 500 should be at the top-middle of the viewport (height of 1000)
            let (x, y) = viewport_transform.transform_point(0.0, 500.0);
            assert!((x-(1024.0+512.0)).abs() < 0.01);
            assert!((y-(1536.0+512.0)).abs() < 0.01);

            // The point 500, 0 should be at the right of the viewport (height of 1000). Pixels are square
            let (x, y) = viewport_transform.transform_point(500.0, 0.0);
            assert!((y-(768.0+512.0)).abs() < 0.01);
            assert!((x-(1792.0+512.0)).abs() < 0.01);
        });
    }

    #[test]
    pub fn window_transform_after_setting_canvas_height_in_big_window_with_scroll() {
        let mut renderer = CanvasRenderer::new();

        executor::block_on(async move {
            // Set the canvas height
            renderer.set_viewport(512.0..1536.0, 512.0..1280.0, 2048.0, 1536.0, 1.0);
            renderer.draw(vec![Draw::ClearCanvas, Draw::CanvasHeight(1000.0)].into_iter()).collect::<Vec<_>>().await;

            // Fetch the viewport transform
            let window_transform = renderer.get_window_transform();

            // The point 0, 500 should be at the top-middle of the viewport (height of 1000)
            let (x, y) = window_transform.transform_point(0.0, 500.0);
            assert!((x-(1024.0)).abs() < 0.01);
            assert!((y-(1536.0)).abs() < 0.01);

            // The point 500, 0 should be at the right of the viewport (height of 1000). Pixels are square
            let (x, y) = window_transform.transform_point(500.0, 0.0);
            assert!((y-(768.0)).abs() < 0.01);
            assert!((x-(1792.0)).abs() < 0.01);
        });
    }

    #[test]
    pub fn window_transform_after_setting_canvas_height_in_big_window_with_scroll_and_scale() {
        let mut renderer = CanvasRenderer::new();

        executor::block_on(async move {
            // Set the canvas height
            renderer.set_viewport(512.0..1536.0, 512.0..1280.0, 2048.0, 1536.0, 2.0);
            renderer.draw(vec![Draw::ClearCanvas, Draw::CanvasHeight(1000.0)].into_iter()).collect::<Vec<_>>().await;

            // Fetch the viewport transform
            let window_transform = renderer.get_window_transform();

            // The point 0, 500 should be at the top-middle of the viewport (height of 1000)
            let (x, y) = window_transform.transform_point(0.0, 500.0);
            assert!((x-(1024.0)).abs() < 0.01);
            assert!((y-(1536.0)).abs() < 0.01);

            // The point 500, 0 should be at the right of the viewport (height of 1000). Pixels are square
            let (x, y) = window_transform.transform_point(500.0, 0.0);
            assert!((y-(768.0)).abs() < 0.01);
            assert!((x-(1792.0)).abs() < 0.01);
        });
    }

    #[test]
    pub fn viewport_transform_for_full_viewport_window() {
        let mut renderer = CanvasRenderer::new();

        renderer.set_viewport(0.0..1024.0, 0.0..768.0, 1024.0, 768.0, 1.0);
        let viewport_transform = renderer.viewport_transform;

        // Top-midpoint is the same
        let (x, y) = viewport_transform.transform_point(0.0, 1.0);
        assert!((x-0.0).abs() < 0.01);
        assert!((y-1.0).abs() < 0.01);

        // Top-left is transformed to give a square aspect ratio
        let (x, y) = viewport_transform.transform_point(-1.0, 1.0);
        assert!((x- -(768.0/1024.0)).abs() < 0.01);
        assert!((y-1.0).abs() < 0.01);
    }

    #[test]
    pub fn window_transform_with_small_viewport_1() {
        let mut renderer = CanvasRenderer::new();

        executor::block_on(async move {
            // Set up a 1:1 transform on the window and a small viewport
            renderer.set_viewport(200.0..300.0, 400.0..450.0, 1024.0, 768.0, 1.0);
            renderer.draw(vec![Draw::ClearCanvas, Draw::CanvasHeight(768.0), Draw::CenterRegion((0.0, 0.0), (1024.0, 768.0))].into_iter()).collect::<Vec<_>>().await;

            // Fetch the viewport transform
            let window_transform    = renderer.get_window_transform();
            let viewport_transform  = renderer.get_viewport_transform();

            // In the window transform, everything should map 1-to-1
            let (x, y) = window_transform.transform_point(0.0, 500.0);
            assert!((x-(0.0)).abs() < 0.01);
            assert!((y-(500.0)).abs() < 0.01);

            let (x, y) = window_transform.transform_point(500.0, 0.0);
            assert!((y-(0.0)).abs() < 0.01);
            assert!((x-(500.0)).abs() < 0.01);

            // The 0,0 point in the viewport should map to 200, 400 on the canvas
            let (x, y) = viewport_transform.transform_point(0.0, 0.0);
            assert!((x-(200.0)).abs() < 0.01);
            assert!((y-(400.0)).abs() < 0.01);
        });
    }

    #[test]
    pub fn window_transform_with_small_viewport_2() {
        let mut renderer = CanvasRenderer::new();

        executor::block_on(async move {
            // Set up a 1:1 transform on the window and a small viewport
            renderer.set_viewport(0.0..300.0, 0.0..450.0, 1024.0, 768.0, 1.0);
            renderer.draw(vec![Draw::ClearCanvas, Draw::CanvasHeight(768.0), Draw::CenterRegion((0.0, 0.0), (1024.0, 768.0))].into_iter()).collect::<Vec<_>>().await;

            // Fetch the viewport transform
            let window_transform    = renderer.get_window_transform();
            let viewport_transform  = renderer.get_viewport_transform();

            // In the window transform, everything should map 1-to-1
            let (x, y) = window_transform.transform_point(0.0, 500.0);
            assert!((x-(0.0)).abs() < 0.01);
            assert!((y-(500.0)).abs() < 0.01);

            let (x, y) = window_transform.transform_point(500.0, 0.0);
            assert!((y-(0.0)).abs() < 0.01);
            assert!((x-(500.0)).abs() < 0.01);

            // The 0,0 point in the viewport should map to 0, 0 on the canvas
            let (x, y) = viewport_transform.transform_point(0.0, 0.0);
            assert!((x-(0.0)).abs() < 0.01);
            assert!((y-(0.0)).abs() < 0.01);
        });
    }
}
