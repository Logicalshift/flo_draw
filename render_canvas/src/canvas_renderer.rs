use super::matrix::*;
use super::fill_state::*;
use super::layer_state::*;
use super::layer_bounds::*;
use super::render_entity::*;
use super::stroke_settings::*;
use super::renderer_core::*;
use super::renderer_layer::*;
use super::renderer_worker::*;
use super::renderer_stream::*;
use super::resource_ids::*;

use flo_render as render;
use flo_render::{RenderTargetType};
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

    /// Vertex buffer used to draw the background quad (if we need to)
    background_vertex_buffer: Option<render::VertexBufferId>,

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
    viewport_size: (f32, f32)
}

impl CanvasRenderer {
    ///
    /// Creates a new canvas renderer
    ///
    pub fn new() -> CanvasRenderer {
        // Create the shared core
        let core = RenderCore {
            frame_starts:               0,
            setup_actions:              vec![],
            layers:                     vec![],
            free_layers:                vec![],
            layer_definitions:          vec![],
            background_color:           render::Rgba8([0, 0, 0, 0]),
            sprites:                    HashMap::new(),
            used_textures:              HashMap::new(),
            render_target_for_texture:  HashMap::new(),
            dynamic_texture_viewport:   HashMap::new(),
            texture_size:               HashMap::new(),
            layer_textures:             HashMap::new(),
            canvas_textures:            HashMap::new(),
            canvas_gradients:           HashMap::new(),
            texture_alpha:              HashMap::new(),
            unused_vertex_buffer:       0,
            free_vertex_buffers:        vec![],
            unused_texture_id:          16,
            free_textures:              vec![]
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
            background_vertex_buffer:   None,
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

        self.window_size                = (window_width, window_height);

        let viewport_width              = x.end-x.start;
        let viewport_height             = y.end-y.start;
        let viewport_width              = if viewport_width < 1.0 { 1.0 } else { viewport_width };
        let viewport_height             = if viewport_height < 1.0 { 1.0 } else { viewport_height };

        self.viewport_origin            = (x.start, y.start);
        self.window_scale               = scale;
        self.viewport_size              = (viewport_width, viewport_height);
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
            render_order:               vec![RenderEntity::SetTransform(canvas::Transform2D::identity())],
            state:                      LayerState {
                is_sprite:          false,
                fill_color:         FillState::Color(render::Rgba8([0, 0, 0, 255])),
                winding_rule:       FillRule::NonZero,
                stroke_settings:    StrokeSettings::new(),
                current_matrix:     canvas::Transform2D::identity(),
                sprite_matrix:      canvas::Transform2D::identity(),
                scale_factor:       1.0,
                blend_mode:         canvas::BlendMode::SourceOver,
                restore_point:      None
            },
            bounds:                     LayerBounds::default(),
            stored_states:              vec![],
            commit_before_rendering:    false,
            commit_after_rendering:     false,
            blend_mode:                 canvas::BlendMode::SourceOver,
            alpha:                      1.0
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
            let mut in_subpath      = false;

            // The last path that was generated
            let mut current_path    = None;

            // The dash pattern that's currently applied
            let mut dash_pattern    = vec![];

            // The active fill state (shader that will be applied to active fills)
            let mut fill_state      = FillState::None;

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
                use canvas::PathOp::*;
                use math::point;

                match draw {
                    StartFrame => {
                        self.core.desync(|core| {
                            core.frame_starts += 1;
                        });
                    }

                    ShowFrame => {
                        self.core.desync(|core| {
                            if core.frame_starts > 0 { 
                                core.frame_starts -= 1;
                            }
                        });
                    }

                    ResetFrame => {
                        self.core.desync(|core| {
                            core.frame_starts = 0;
                        });
                    }

                    // Begins a new path
                    Path(NewPath) => {
                        current_path = None;
                        in_subpath   = false;
                        path_builder = Some(path::Path::builder());
                    }

                    // Move to a new point
                    Path(Move(x, y)) => {
                        if in_subpath {
                            path_builder.as_mut().map(|builder| builder.end(false));
                        }
                        path_builder.get_or_insert_with(|| path::Path::builder())
                            .begin(point(x, y));
                        in_subpath = true;
                    }

                    // Line to point
                    Path(Line(x, y)) => {
                        if in_subpath {
                            path_builder.get_or_insert_with(|| path::Path::builder())
                                .line_to(point(x, y));
                        } else {
                            path_builder.get_or_insert_with(|| path::Path::builder())
                                .begin(point(x, y));
                            in_subpath = true;
                        }
                    }

                    // Bezier curve to point
                    Path(BezierCurve(((cp1x, cp1y), (cp2x, cp2y)), (px, py))) => {
                        if in_subpath {
                            path_builder.get_or_insert_with(|| path::Path::builder())
                                .cubic_bezier_to(point(cp1x, cp1y), point(cp2x, cp2y), point(px, py));
                        } else {
                            path_builder.get_or_insert_with(|| path::Path::builder())
                                .begin(point(px, py));
                            in_subpath = true;
                        }
                    }

                    // Closes the current path
                    Path(ClosePath) => {
                        path_builder.get_or_insert_with(|| path::Path::builder())
                            .end(true);
                        in_subpath = false;
                    }

                    // Fill the current path
                    Fill => {
                        // Update the active path if the builder exists
                        if let Some(mut path_builder) = path_builder.take() {
                            if in_subpath { path_builder.end(false); }
                            current_path = Some(path_builder.build());
                        }

                        // Publish the fill job to the tessellators
                        if let Some(path) = &current_path {
                            let path                = path.clone();
                            let layer_id            = self.current_layer;
                            let entity_id           = self.next_entity_id;
                            let viewport_height     = self.viewport_size.1;
                            let active_transform    = &self.active_transform;
                            let dash_pattern        = &mut dash_pattern;
                            let fill_state          = &mut fill_state;

                            self.next_entity_id += 1;

                            let job         = core.sync(move |core| {
                                let layer               = core.layer(layer_id);

                                // Update the transformation matrix
                                layer.update_transform(active_transform);

                                // Rendering in a blend mode other than source over sets the 'commit before rendering' flag for this layer
                                if layer.state.blend_mode != canvas::BlendMode::SourceOver {
                                    layer.commit_before_rendering = true;
                                }

                                // If the shader state has changed, generate the operations needed to use that shader state
                                if *fill_state != layer.state.fill_color {
                                    // Update the active fill state to match that of the layer
                                    match layer.state.fill_color {
                                        FillState::None | FillState::Color(_) => { 
                                            layer.render_order.push(RenderEntity::SetFlatColor);
                                        }

                                        FillState::Texture(texture_id, matrix, repeat, alpha) => {
                                            // Finish/get the render texture
                                            if let Some(render_texture) = core.texture_for_rendering(texture_id) {
                                                // Increase the usage count for this texture
                                                core.used_textures.get_mut(&render_texture)
                                                    .map(|usage_count| *usage_count += 1);

                                                // Add to the layer
                                                core.layer(layer_id).render_order.push(RenderEntity::SetFillTexture(render_texture, matrix, repeat, alpha));
                                            } else {
                                                // Texture is not set up
                                                core.layer(layer_id).render_order.push(RenderEntity::SetFlatColor);
                                            }
                                        }

                                        FillState::LinearGradient(gradient_id, matrix, repeat, alpha) => {
                                            // Finish/get the texture for the gradient
                                            if let Some(gradient_texture) = core.gradient_for_rendering(gradient_id) {
                                                // Increase the usage count for the texture
                                                core.used_textures.get_mut(&gradient_texture)
                                                    .map(|usage_count| *usage_count += 1);

                                                // Add to the layer
                                                core.layer(layer_id).render_order.push(RenderEntity::SetFillGradient(gradient_texture, matrix, repeat, alpha));
                                            } else {
                                                // Gradient is not set up
                                                core.layer(layer_id).render_order.push(RenderEntity::SetFlatColor);
                                            }
                                        }
                                    }


                                    *dash_pattern   = vec![];
                                    *fill_state     = core.layer(layer_id).state.fill_color.clone();
                                } else if *dash_pattern != vec![] {
                                    // Ensure there's no dash pattern
                                    layer.render_order.push(RenderEntity::SetFlatColor);
                                    *dash_pattern   = vec![];
                                    *fill_state     = layer.state.fill_color.clone();
                                }

                                // Create the render entity in the tessellating state
                                let layer               = core.layer(layer_id);
                                let scale_factor        = layer.state.tolerance_scale_factor(viewport_height);
                                let color               = layer.state.fill_color.clone();
                                let fill_rule           = layer.state.winding_rule;
                                let entity_index        = layer.render_order.len();
                                let transform           = *active_transform;

                                // When drawing to the erase layer (DesintationOut blend mode), all colour components are alpha components
                                let color               = if layer.state.blend_mode == canvas::BlendMode::DestinationOut { color.all_channel_alpha() } else { color };

                                layer.render_order.push(RenderEntity::Tessellating(entity_id));

                                let entity          = LayerEntityRef { layer_id, entity_index, entity_id };

                                // Create the canvas job
                                CanvasJob::Fill { path, fill_rule, color, scale_factor, transform, entity }
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
                        if let Some(mut path_builder) = path_builder.take() {
                            if in_subpath { path_builder.end(false); }
                            current_path = Some(path_builder.build());
                        }

                        // Publish the job to the tessellators
                        if let Some(path) = &current_path {
                            let path                = path.clone();
                            let layer_id            = self.current_layer;
                            let entity_id           = self.next_entity_id;
                            let viewport_height     = self.viewport_size.1;
                            let active_transform    = &self.active_transform;
                            let dash_pattern        = &mut dash_pattern;
                            let fill_state          = &mut fill_state;

                            self.next_entity_id += 1;

                            let job         = core.sync(move |core| {
                                let layer               = core.layer(layer_id);

                                // Rendering in a blend mode other than source over sets the 'commit before rendering' flag for this layer
                                if layer.state.blend_mode != canvas::BlendMode::SourceOver {
                                    layer.commit_before_rendering = true;
                                }

                                // Update the transformation matrix
                                layer.update_transform(active_transform);

                                // Reset the fill state to 'flat colour' if needed
                                match fill_state {
                                    FillState::None     | 
                                    FillState::Color(_) => { }
                                    _                   => { layer.render_order.push(RenderEntity::SetFlatColor) }
                                }

                                *fill_state = FillState::None;

                                // Apply the dash pattern, if it's different
                                if *dash_pattern != layer.state.stroke_settings.dash_pattern {
                                    layer.render_order.push(RenderEntity::SetDashPattern(layer.state.stroke_settings.dash_pattern.clone()));
                                    *dash_pattern = layer.state.stroke_settings.dash_pattern.clone();
                                }

                                // Create the render entity in the tessellating state
                                let scale_factor        = layer.state.tolerance_scale_factor(viewport_height);
                                let mut stroke_options  = layer.state.stroke_settings.clone();
                                let entity_index        = layer.render_order.len();
                                let transform           = *active_transform;

                                // When drawing to the erase layer (DesintationOut blend mode), all colour components are alpha components
                                let color                   = stroke_options.stroke_color;
                                stroke_options.stroke_color = if layer.state.blend_mode == canvas::BlendMode::DestinationOut { render::Rgba8([color.0[3], color.0[3], color.0[3], color.0[3]]) } else { color };

                                layer.render_order.push(RenderEntity::Tessellating(entity_id));

                                let entity          = LayerEntityRef { layer_id, entity_index, entity_id };

                                // Create the canvas job
                                CanvasJob::Stroke { path, stroke_options, scale_factor, transform, entity }
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
                        core.sync(|core| core.layer(self.current_layer).state.fill_color = FillState::Color(Self::render_color(color)));
                    }

                    // Set a fill texture
                    FillTexture(texture_id, (x1, y1), (x2, y2)) => {
                        core.sync(|core| {
                            let alpha               = core.texture_alpha.get(&texture_id).cloned().unwrap_or(1.0);
                            let layer               = core.layer(self.current_layer);

                            layer.state.fill_color  = FillState::texture_fill(texture_id, x1, y1, x2, y2, alpha)
                        });
                    }

                    // Set a fill gradient
                    FillGradient(gradient_id, (x1, y1), (x2, y2)) => {
                        core.sync(|core| {
                            let layer               = core.layer(self.current_layer);

                            layer.state.fill_color  = FillState::linear_gradient_fill(gradient_id, x1, y1, x2, y2);
                        });
                    }

                    // Transforms the existing fill
                    FillTransform(transform) => {
                        core.sync(|core| {
                            let layer               = core.layer(self.current_layer);

                            let transform           = transform.invert().unwrap_or_else(|| canvas::Transform2D::identity());
                            layer.state.fill_color  = layer.state.fill_color.transform(&transform);
                        });
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
                                SourceOver      => render::BlendMode::SourceOver,
                                DestinationOver => render::BlendMode::DestinationOver,
                                DestinationOut  => render::BlendMode::DestinationOut,

                                SourceIn        => render::BlendMode::SourceIn,
                                SourceOut       => render::BlendMode::SourceOut,
                                DestinationIn   => render::BlendMode::DestinationIn,
                                SourceAtop      => render::BlendMode::SourceATop,
                                DestinationAtop => render::BlendMode::DestinationATop,

                                Multiply        => render::BlendMode::Multiply,
                                Screen          => render::BlendMode::Screen,

                                // TODO: these are not supported yet (they might require explicit shader support)
                                Darken          => render::BlendMode::SourceOver,
                                Lighten         => render::BlendMode::SourceOver,
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
                        core.sync(|core| {
                            let layer           = core.layer(self.current_layer);

                            // Render the sprite
                            layer.render_order.push(RenderEntity::DisableClipping);
                        })
                    }

                    // Clip to the currently set path
                    Clip => {
                        // Update the active path if the builder exists
                        if let Some(mut path_builder) = path_builder.take() {
                            if in_subpath { path_builder.end(false); }
                            current_path = Some(path_builder.build());
                        }

                        // Publish the fill job to the tessellators
                        if let Some(path) = &current_path {
                            let path                = path.clone();
                            let layer_id            = self.current_layer;
                            let entity_id           = self.next_entity_id;
                            let viewport_height     = self.viewport_size.1;
                            let active_transform    = &self.active_transform;

                            self.next_entity_id += 1;

                            let job         = core.sync(move |core| {
                                let layer               = core.layer(layer_id);

                                // Update the transformation matrix
                                layer.update_transform(active_transform);

                                // Create the render entity in the tessellating state
                                let scale_factor        = layer.state.tolerance_scale_factor(viewport_height);
                                let color               = render::Rgba8([255, 255, 255, 255]);
                                let fill_rule           = layer.state.winding_rule;
                                let entity_index        = layer.render_order.len();
                                let transform           = *active_transform;

                                // Update the clipping path and enable clipping
                                layer.render_order.push(RenderEntity::Tessellating(entity_id));

                                let entity          = LayerEntityRef { layer_id, entity_index, entity_id };

                                // Create the canvas job
                                CanvasJob::Clip { path, fill_rule, color, scale_factor, transform, entity }
                            });

                            pending_jobs.push(job);
                            if pending_jobs.len() >= batch_size {
                                job_publisher.publish(pending_jobs).await;
                                pending_jobs = vec![];
                            }
                        }
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
                                core.layer(layer_id).pop_state();
                            }
                        })
                    }

                    // Clears the canvas entirely
                    ClearCanvas(background) => {
                        //todo!("Stop any incoming tessellated data for this layer");
                        //todo!("Mark vertex buffers as freed");

                        fill_state      = FillState::None;
                        dash_pattern    = vec![];
                        current_path    = None;
                        path_builder    = None;
                        in_subpath      = false;

                        core.sync(|core| {
                            // Release the textures
                            let old_textures = mem::take(&mut core.canvas_textures);

                            for (_canvas_id, render_id) in old_textures.into_iter() {
                                let render_id = (&render_id).into();
                                core.used_textures.get_mut(&render_id).map(|usage_count| *usage_count -= 1);
                            }

                            // Release the existing layers
                            let old_layers = mem::take(&mut core.layers);

                            for layer_id in old_layers {
                                let layer = core.release_layer_handle(layer_id);
                                core.free_layer_entities(layer);
                            }

                            // Release the sprites
                            let old_sprites = mem::take(&mut core.sprites);

                            for (_sprite_id, layer_id) in old_sprites {
                                let layer = core.release_layer_handle(layer_id);
                                core.free_layer_entities(layer);
                            }

                            // Set the background colour for when we start rendering
                            core.background_color   = Self::render_color(background);

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
                    Layer(canvas::LayerId(layer_id)) => {
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
                    LayerBlend(canvas::LayerId(layer_id), blend_mode) => {
                        core.sync(move |core| {
                            let layer_id = layer_id as usize;

                            if layer_id < core.layers.len() {
                                // Fetch the layer
                                let layer_handle    = core.layers[layer_id];
                                let layer           = core.layer(layer_handle);

                                // Update the blend mode and set the layer's 'commit' mode
                                layer.blend_mode    = blend_mode;
                                if blend_mode != canvas::BlendMode::SourceOver {
                                    // Need to commit before to stop whatever is under the layer from having the blend mode applied to it, and after to apply the blend mode
                                    layer.commit_before_rendering   = true;
                                    layer.commit_after_rendering    = true;
                                }
                            }
                        });
                    }

                    // Sets the alpha blend mode for a particular layer
                    LayerAlpha(canvas::LayerId(layer_id), layer_alpha) => {
                        core.sync(move |core| {
                            let layer_id = layer_id as usize;

                            if layer_id < core.layers.len() {
                                // Fetch the layer
                                let layer_handle    = core.layers[layer_id];
                                let layer           = core.layer(layer_handle);

                                let layer_alpha     = f32::max(0.0, f32::min(1.0, layer_alpha));

                                // Update the alpha value and set the layer's 'commit' mode
                                layer.alpha    = layer_alpha as _;
                                if layer_alpha < 1.0 {
                                    layer.commit_before_rendering   = true;
                                    layer.commit_after_rendering    = true;
                                }
                            }
                        });
                    }

                    // Clears the current layer
                    ClearLayer | ClearSprite => {
                        fill_state      = FillState::None;
                        dash_pattern    = vec![];
                        current_path    = None;
                        path_builder    = None;
                        in_subpath      = false;

                        core.sync(|core| {
                            // Create a new layer
                            let mut layer   = Self::create_default_layer();

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

                    ClearAllLayers => {
                        fill_state      = FillState::None;
                        dash_pattern    = vec![];
                        current_path    = None;
                        path_builder    = None;
                        in_subpath      = false;

                        core.sync(|core| {
                            let handles = core.layers.clone();

                            for handle in handles.into_iter() {
                                // Sprite layers are left alone
                                if core.layer(self.current_layer).state.is_sprite {
                                    continue;
                                }

                                // Create a new layer
                                let mut layer   = Self::create_default_layer();

                                // Swap into the layer list to replace the old one
                                mem::swap(core.layer(handle), &mut layer);

                                // Free the data for the current layer
                                core.free_layer_entities(layer);
                            }
                        });
                    }

                    SwapLayers(canvas::LayerId(layer1), canvas::LayerId(layer2)) => {
                        if layer1 != layer2 {
                            core.sync(move |core| {
                                // Create layers so we can swap with arbitrary layers
                                let max_layer_id = u64::max(layer1, layer2) as usize;
                                while core.layers.len() <= max_layer_id  {
                                    let new_layer = Self::create_default_layer();
                                    let new_layer = core.allocate_layer_handle(new_layer);
                                    core.layers.push(new_layer);
                                }

                                // Swap the two layers in the core
                                let LayerHandle(handle1) = core.layers[layer1 as usize];
                                let LayerHandle(handle2) = core.layers[layer2 as usize];

                                if handle1 != handle2 {
                                    core.layer_definitions.swap(handle1 as usize, handle2 as usize);
                                }
                            });
                        }

                        todo!()
                    }

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

                    // Creates or replaces a texture
                    Texture(texture_id, canvas::TextureOp::Create(canvas::TextureSize(width, height), canvas::TextureFormat::Rgba)) => {
                        core.sync(|core| {
                            // If the texture ID was previously in use, reduce the usage count
                            let render_texture = if let Some(old_render_texture) = core.canvas_textures.get(&texture_id) {
                                let old_render_texture  = old_render_texture.into();
                                let usage_count         = core.used_textures.get_mut(&old_render_texture);

                                if usage_count == Some(&mut 1) {
                                    // Leave the usage count as is and reallocate the existing texture
                                    // The 1 usage is the rendered version of this texture
                                    old_render_texture
                                } else {
                                    // Reduce the usage count
                                    usage_count.map(|usage_count| *usage_count -=1);

                                    // Allocate a new texture
                                    core.allocate_texture()
                                }
                            } else {
                                // Unused texture ID: allocate a new texture
                                core.allocate_texture()
                            };

                            // Add this as a texture with a usage count of 1
                            core.canvas_textures.insert(texture_id, RenderTexture::Loading(render_texture));
                            core.used_textures.insert(render_texture, 1);
                            core.texture_size.insert(render_texture, render::Size2D(width as _, height as _));

                            // Create the texture in the setup actions
                            core.setup_actions.push(render::RenderAction::CreateTextureBgra(render_texture, render::Size2D(width as _, height as _)));
                        });
                    }

                    // Release an existing texture
                    Texture(texture_id, canvas::TextureOp::Free) => {
                        core.sync(|core| {
                            // If the texture ID was previously in use, reduce the usage count
                            if let Some(old_render_texture) = core.canvas_textures.get(&texture_id) {
                                let old_render_texture = old_render_texture.into();
                                core.used_textures.get_mut(&old_render_texture)
                                    .map(|usage_count| *usage_count -=1);
                            }

                            // Unmap the texture
                            core.canvas_textures.remove(&texture_id);
                        });
                    }

                    // Updates an existing texture
                    Texture(texture_id, canvas::TextureOp::SetBytes(canvas::TexturePosition(x, y), canvas::TextureSize(width, height), bytes)) => {
                        core.sync(|core| {
                            // Create a canvas renderer job that will write these bytes to the texture
                            if let Some(render_texture) = core.canvas_textures.get(&texture_id) {
                                let mut render_texture = *render_texture;

                                // If the texture has one used count and is in a 'ready' state, switch it back to 'loading' (nothing has rendered it)
                                if let RenderTexture::Ready(render_texture_id) = &render_texture {
                                    if core.used_textures.get(render_texture_id) == Some(&1) {
                                        core.canvas_textures.insert(texture_id, RenderTexture::Loading(*render_texture_id));
                                        render_texture = RenderTexture::Loading(*render_texture_id);
                                    }
                                }

                                // The texture is updated in a setup action
                                match render_texture {
                                    RenderTexture::Ready(render_texture)    => {
                                        // Generate a copy of the texture and write to that instead ('Ready' textures are already rendered elsewhere)
                                        let copy_texture_id = core.allocate_texture();

                                        // Stop using the initial texture, and create a new copy that's 'Loading'
                                        core.used_textures.get_mut(&render_texture).map(|usage_count| *usage_count -= 1);
                                        core.used_textures.insert(copy_texture_id, 1);
                                        core.canvas_textures.insert(texture_id, RenderTexture::Loading(copy_texture_id));

                                        // Generate a copy
                                        core.texture_size.insert(copy_texture_id, core.texture_size.get(&render_texture).unwrap().clone());
                                        core.setup_actions.push(render::RenderAction::CopyTexture(render_texture, copy_texture_id));

                                        // Update the data in the copy
                                        core.setup_actions.push(render::RenderAction::WriteTextureData(copy_texture_id, render::Position2D(x as _, y as _), render::Position2D((x+width) as _, (y+height) as _), bytes));
                                    }

                                    RenderTexture::Loading(render_texture)  => {
                                        // Use the existing texture
                                        core.setup_actions.push(render::RenderAction::WriteTextureData(render_texture, render::Position2D(x as _, y as _), render::Position2D((x+width) as _, (y+height) as _), bytes));
                                    }
                                }
                            }
                        });
                    }

                    // Render a texture from a sprite
                    Texture(texture_id, canvas::TextureOp::SetFromSprite(sprite_id, canvas::SpriteBounds(canvas::SpritePosition(x, y), canvas::SpriteSize(w, h)))) => {
                        core.sync(|core| {
                            // Specify this as a texture that needs to be loaded by rendering from a layer
                            if let (Some(render_texture), Some(sprite_layer_handle)) = (core.canvas_textures.get(&texture_id), core.sprites.get(&sprite_id)) {
                                let mut render_texture  = *render_texture;
                                let sprite_layer_handle = *sprite_layer_handle;

                                // If the texture has one used count and is in a 'ready' state, switch it back to 'loading' (nothing has rendered it)
                                if let RenderTexture::Ready(render_texture_id) = &render_texture {
                                    if core.used_textures.get(render_texture_id) == Some(&1) {
                                        core.canvas_textures.insert(texture_id, RenderTexture::Loading(*render_texture_id));
                                        render_texture = RenderTexture::Loading(*render_texture_id);
                                    }
                                }

                                // This texture needs to be marked to be rendered after the setup is completed
                                let texture_id = match render_texture {
                                    RenderTexture::Ready(render_texture)    => {
                                        // Create a blank texture, and move back to the loading state
                                        let new_texture_id = core.allocate_texture();

                                        // Stop using the initial texture, and create a new copy that's 'Loading'
                                        core.used_textures.get_mut(&render_texture).map(|usage_count| *usage_count -= 1);
                                        core.used_textures.insert(new_texture_id, 1);
                                        core.canvas_textures.insert(texture_id, RenderTexture::Ready(new_texture_id));

                                        // Generate a copy
                                        core.texture_size.insert(new_texture_id, core.texture_size.get(&render_texture).unwrap().clone());
                                        core.setup_actions.push(render::RenderAction::CopyTexture(render_texture, new_texture_id));

                                        // Write to the new texture
                                        new_texture_id
                                    }

                                    RenderTexture::Loading(render_texture)  => {
                                        // Use the existing texture
                                        core.canvas_textures.insert(texture_id, RenderTexture::Ready(render_texture));
                                        render_texture
                                    }
                                };

                                // Cause the stream to render the sprite to the texture at the start of the next frame
                                core.layer_textures.insert(texture_id, TextureRenderRequest::FromSprite(texture_id, sprite_layer_handle, canvas::SpriteBounds(canvas::SpritePosition(x, y), canvas::SpriteSize(w, h))));
                            }
                        });
                    }

                    // Render a texture from a sprite, updating it dynamically as the canvas resolution changes
                    Texture(texture_id, canvas::TextureOp::CreateDynamicSprite(sprite_id, sprite_bounds, canvas_size)) => {
                        core.sync(|core| {
                            if let Some(sprite_layer_handle) = core.sprites.get(&sprite_id) {
                                let sprite_layer_handle = *sprite_layer_handle;
                                let transform           = core.layer(self.current_layer).state.current_matrix;

                                // If the texture ID was previously in use, reduce the usage count
                                let render_texture_id = if let Some(old_render_texture) = core.canvas_textures.get(&texture_id) {
                                    let old_render_texture  = old_render_texture.into();
                                    let usage_count         = core.used_textures.get_mut(&old_render_texture);

                                    if usage_count == Some(&mut 1) {
                                        // Leave the usage count as is and reallocate the existing texture
                                        // The 1 usage is the rendered version of this texture
                                        old_render_texture
                                    } else {
                                        // Reduce the usage count
                                        usage_count.map(|usage_count| *usage_count -=1);

                                        // Allocate a new texture
                                        core.allocate_texture()
                                    }
                                } else {
                                    // Unused texture ID: allocate a new texture
                                    core.allocate_texture()
                                };

                                // Add this as a texture with a usage count of 1
                                core.canvas_textures.insert(texture_id, RenderTexture::Loading(render_texture_id));
                                core.used_textures.insert(render_texture_id, 1);
                                core.texture_size.insert(render_texture_id, render::Size2D(1 as _, 1 as _));
                                core.dynamic_texture_viewport.remove(&render_texture_id);

                                // Specify as a dynamic texture
                                core.layer_textures.insert(render_texture_id, TextureRenderRequest::DynamicTexture(render_texture_id, sprite_layer_handle, sprite_bounds, canvas_size, transform));
                            }
                        });
                    },

                    // Sets the transparency to use when drawing a particular texture
                    Texture(texture_id, canvas::TextureOp::FillTransparency(alpha)) => {
                        self.core.sync(|core| {
                            core.texture_alpha.insert(texture_id, alpha);
                            let layer                   = core.layer(self.current_layer);

                            if layer.state.fill_color.texture_id() == Some(texture_id) {
                                layer.state.fill_color  = layer.state.fill_color.with_texture_alpha(alpha);
                            }
                        });
                    }

                    // Performs a font operation
                    Font(_, _) => {
                        // Fonts aren't directly rendered by the canvas renderer (need a helper to convert to textures or outlines)
                    },

                    // Draws some text in a particular font
                    DrawText(_, _, _, _) => {
                        // Fonts aren't directly rendered by the canvas renderer (need a helper to convert to textures or outlines)
                    },

                    BeginLineLayout(_, _, _) => {
                        // Fonts aren't directly rendered by the canvas renderer (need a helper to convert to textures or outlines)
                    },

                    DrawLaidOutText => {
                        // Fonts aren't directly rendered by the canvas renderer (need a helper to convert to textures or outlines)
                    },
                    
                    Gradient(gradient_id, canvas::GradientOp::Create(initial_colour)) => {
                        // Start the gradient definition from scratch
                        self.core.sync(move |core| {
                            core.canvas_gradients.insert(gradient_id, RenderGradient::Defined(vec![canvas::GradientOp::Create(initial_colour)]));
                        });
                    }

                    Gradient(gradient_id, canvas::GradientOp::AddStop(pos, stop_colour)) => {
                        // Continue an existing gradient definition
                        self.core.sync(move |core| {
                            use canvas::GradientOp::AddStop;

                            match core.canvas_gradients.get_mut(&gradient_id) {
                                Some(RenderGradient::Defined(defn)) => {
                                    // Gradient has not yet been mapped to a texture
                                    defn.push(AddStop(pos, stop_colour))
                                }

                                Some(RenderGradient::Ready(_, defn)) => {
                                    // Gradient has been mapped to a texture (continue defining it as a new texture)
                                    let mut defn = defn.clone();
                                    defn.push(AddStop(pos, stop_colour));
                                    core.canvas_gradients.insert(gradient_id, RenderGradient::Defined(defn));
                                }

                                None => { }
                            }
                        });
                    }
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
                for (entity, operation, details) in result_list {
                    // Store each result in the core
                    core.sync(|core| core.store_job_result(entity, operation, details));
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
        // See if rendering is suspended (we just load vertex buffers if it is)
        let rendering_suspended = self.core.sync(|core| core.frame_starts > 0);

        // Set up the initial set of rendering actions
        let viewport_transform  = self.viewport_transform;
        let viewport_size       = render::Size2D(self.viewport_size.0 as usize, self.viewport_size.1 as usize);
        let viewport_matrix     = transform_to_matrix(&self.viewport_transform);
        let mut initialise      = if rendering_suspended {
            vec![]
        } else { 
            vec![
                render::RenderAction::SelectRenderTarget(MAIN_RENDER_TARGET),
                render::RenderAction::BlendMode(render::BlendMode::SourceOver),
                render::RenderAction::Clear(render::Rgba8([0, 0, 0, 0])),
                render::RenderAction::SetTransform(viewport_matrix),
            ]
        };

        // Initialise the default render target
        initialise.insert(0, render::RenderAction::CreateRenderTarget(MAIN_RENDER_TARGET, MAIN_RENDER_TEXTURE, 
            render::Size2D(self.viewport_size.0 as usize, self.viewport_size.1 as usize),
            RenderTargetType::MultisampledTexture));

        // And the 'clip mask' render surface (render target 2, texture 2)
        initialise.insert(0, render::RenderAction::CreateRenderTarget(CLIP_RENDER_TARGET, CLIP_RENDER_TEXTURE,
            render::Size2D(self.viewport_size.0 as usize, self.viewport_size.1 as usize),
            RenderTargetType::MonochromeMultisampledTexture));

        // When finished, render the MSAA buffer to the main framebuffer
        let finalize            = if rendering_suspended {
            vec![]
        } else {
            vec![
                render::RenderAction::RenderToFrameBuffer,
                render::RenderAction::BlendMode(render::BlendMode::SourceOver),
                render::RenderAction::SetTransform(render::Matrix::identity()),
                // Note that the framebuffer region can be updated by the renderer stream (or this instruction can be removed): see `clip_draw_framebuffer()` in renderer_stream.rs
                render::RenderAction::DrawFrameBuffer(MAIN_RENDER_TARGET, render::FrameBufferRegion::default(), render::Alpha(1.0)),
                render::RenderAction::ShowFrameBuffer,

                render::RenderAction::FreeRenderTarget(MAIN_RENDER_TARGET),
                render::RenderAction::FreeRenderTarget(CLIP_RENDER_TARGET),
                render::RenderAction::FreeTexture(MAIN_RENDER_TEXTURE),
                render::RenderAction::FreeTexture(CLIP_RENDER_TEXTURE),
            ]
        };

        // The render stream needs a vertex buffer to render the background to, so make sure that's allocated
        let background_vertex_buffer = match self.background_vertex_buffer {
            Some(buffer_id) => buffer_id,
            None            => {
                // Allocate the buffer
                let buffer_id                   = self.core.sync(|core| core.allocate_vertex_buffer());
                let buffer_id                   = render::VertexBufferId(buffer_id);
                self.background_vertex_buffer   = Some(buffer_id);
                buffer_id
            }
        };

        // We need to process the instructions waiting to set up textures
        let setup_textures = self.core.sync(|core| {
            let mut textures    = vec![];
            let viewport_size   = self.viewport_size;

            for (_, render_request) in mem::take(&mut core.layer_textures) {
                use self::TextureRenderRequest::*;
                match &render_request {
                    FromSprite(_, _, _) => {
                        // These are always rendered
                        textures.push(render_request);
                    },

                    DynamicTexture(texture_id, _, _, _, _) => {
                        let texture_id = *texture_id;

                        if core.dynamic_texture_viewport.get(&texture_id) != Some(&viewport_size) {
                            // These are rendered if the viewport or sprite has changed since the last time
                            textures.push(render_request);

                            // Update the viewport data so this isn't re-rendered until it changes
                            core.dynamic_texture_viewport.insert(texture_id, viewport_size);
                        }

                        // Put back on the request list so we re-render this texture
                        core.layer_textures.insert(texture_id, render_request);
                    }
                }
            }

            textures
        });

        // Start processing the drawing instructions
        let core                = Arc::clone(&self.core);
        let processing          = self.process_drawing(drawing);

        // Return a stream of results from processing the drawing
        RenderStream::new(core, rendering_suspended, processing, viewport_transform, viewport_size, background_vertex_buffer, initialise, setup_textures, finalize)
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
            renderer.draw(vec![Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 0.0)), Draw::CanvasHeight(1000.0)].into_iter()).collect::<Vec<_>>().await;

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
            renderer.draw(vec![Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 0.0)), Draw::CanvasHeight(1000.0)].into_iter()).collect::<Vec<_>>().await;

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
            renderer.draw(vec![Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 0.0)), Draw::CanvasHeight(1000.0)].into_iter()).collect::<Vec<_>>().await;

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
            renderer.draw(vec![Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 0.0)), Draw::CanvasHeight(1000.0)].into_iter()).collect::<Vec<_>>().await;

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
            renderer.draw(vec![Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 0.0)), Draw::CanvasHeight(1000.0)].into_iter()).collect::<Vec<_>>().await;

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
            renderer.draw(vec![Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 0.0)), Draw::CanvasHeight(1000.0)].into_iter()).collect::<Vec<_>>().await;

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
            renderer.draw(vec![Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 0.0)), Draw::CanvasHeight(1000.0)].into_iter()).collect::<Vec<_>>().await;

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
            renderer.draw(vec![Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 0.0)), Draw::CanvasHeight(768.0), Draw::CenterRegion((0.0, 0.0), (1024.0, 768.0))].into_iter()).collect::<Vec<_>>().await;

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
            renderer.draw(vec![Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 0.0)), Draw::CanvasHeight(768.0), Draw::CenterRegion((0.0, 0.0), (1024.0, 768.0))].into_iter()).collect::<Vec<_>>().await;

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
