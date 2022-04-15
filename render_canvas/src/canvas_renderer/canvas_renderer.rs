use crate::matrix::*;
use crate::renderer_core::*;
use crate::renderer_worker::*;
use crate::renderer_stream::*;
use crate::resource_ids::*;
use crate::dynamic_texture_state::*;
use crate::layer_handle::*;
use crate::texture_render_request::*;

use super::tessellate_build_path::*;

use flo_render as render;
use flo_render::{RenderTargetType};
use flo_canvas as canvas;
use flo_stream::*;

use ::desync::*;

use futures::prelude::*;
use num_cpus;

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
    pub (super) core: Arc<Desync<RenderCore>>,

    /// Vertex buffer used to draw the background quad (if we need to)
    background_vertex_buffer: Option<render::VertexBufferId>,

    /// The layer that the next drawing instruction will apply to
    pub (super) current_layer: LayerHandle,

    /// The viewport transformation (this makes for rectangular pixels with the bottom of the window at 0, -1 and the top at 0, 1)
    viewport_transform: canvas::Transform2D,

    /// The inverse of the viewport transformation
    inverse_viewport_transform: canvas::Transform2D,

    /// The currently active transformation
    pub (super) active_transform: canvas::Transform2D,

    /// The transforms pushed to the stack when PushState was called
    pub (super) transform_stack: Vec<canvas::Transform2D>,

    /// The next ID to assign to an entity for tessellation
    pub (super) next_entity_id: usize,

    /// The width and size of the window overall
    pub (super) window_size: (f32, f32),

    /// The scale factor of the window
    pub (super) window_scale: f32,

    /// The origin of the viewport
    viewport_origin: (f32, f32),

    /// The width and size of the viewport we're rendering to
    pub (super) viewport_size: (f32, f32)
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
            dynamic_texture_state:      HashMap::new(),
            texture_size:               HashMap::new(),
            layer_textures:             vec![],
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
    /// Tessellates a drawing to the layers in this renderer
    ///
    fn tessellate<'a, DrawIter: 'a+Iterator<Item=canvas::Draw>>(&'a mut self, drawing: DrawIter, job_publisher: SinglePublisher<Vec<CanvasJob>>) -> impl 'a+Future<Output=()> {
        async move {
            let core                = Arc::clone(&self.core);
            let mut job_publisher   = job_publisher;
            let mut pending_jobs    = vec![];

            // The current path that is being built up
            let mut path_state      = PathState::default();

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

                match draw {
                    StartFrame                                  => self.tes_start_frame(),
                    ShowFrame                                   => self.tes_show_frame(),
                    ResetFrame                                  => self.tes_reset_frame(),

                    Path(NewPath)                               => path_state.tes_new_path(),
                    Path(Move(x, y))                            => path_state.tes_move(x, y),
                    Path(Line(x, y))                            => path_state.tes_line(x, y),
                    Path(BezierCurve((cp1, cp2), p))            => path_state.tes_bezier_curve(cp1, cp2, p),
                    Path(ClosePath)                             => path_state.tes_close_path(),

                    Fill                                        => self.tes_fill(&mut path_state, &mut job_publisher, &mut pending_jobs).await,
                    Stroke                                      => self.tes_stroke(&mut path_state, &mut job_publisher, &mut pending_jobs).await,

                    LineWidth(width)                            => self.tes_line_width(width),
                    LineWidthPixels(pixel_width)                => self.tes_line_width_pixels(pixel_width),
                    LineJoin(join_type)                         => self.tes_line_join(join_type),
                    LineCap(cap_type)                           => self.tes_line_cap(cap_type),
                    WindingRule(winding_rule)                   => self.tes_winding_rule(winding_rule),
                    NewDashPattern                              => self.tes_new_dash_pattern(),
                    DashLength(length)                          => self.tes_dash_length(length),
                    DashOffset(offset)                          => self.tes_dash_offset(offset),
                    FillColor(color)                            => self.tes_fill_color(color),
                    FillTexture(texture_id, min, max)           => self.tes_fill_texture(texture_id, min, max),
                    FillGradient(gradient_id, min, max)         => self.tes_fill_gradient(gradient_id, min, max),
                    FillTransform(transform)                    => self.tes_fill_transform(transform),
                    StrokeColor(color)                          => self.tes_stroke_color(color),
                    BlendMode(blend_mode)                       => self.tes_blend_mode(blend_mode),

                    IdentityTransform                           => self.tes_identity_transform(), 
                    CanvasHeight(height)                        => self.tes_canvas_height(height),
                    CenterRegion(min, max)                      => self.tes_center_region(min, max),
                    MultiplyTransform(transform)                => self.tes_multiply_transform(transform),

                    Unclip                                      => self.tes_unclip(),
                    Clip                                        => self.tes_clip(&mut path_state, &mut job_publisher, &mut pending_jobs).await,

                    Store                                       => self.tes_store(),
                    Restore                                     => self.tes_restore(),
                    FreeStoredBuffer                            => self.tes_free_stored_buffer(),
                    PushState                                   => self.tes_push_state(),
                    PopState                                    => self.tes_pop_state(),

                    ClearCanvas(background)                     => self.tes_clear_canvas(background, &mut path_state),
                    Layer(layer_id)                             => self.tes_layer(layer_id),
                    LayerBlend(layer_id, blend_mode)            => self.tes_layer_blend(layer_id, blend_mode),
                    LayerAlpha(layer_id, layer_alpha)           => self.tes_layer_alpha(layer_id, layer_alpha),
                    ClearLayer                                  => self.tes_clear_layer(&mut path_state), 
                    ClearAllLayers                              => self.tes_clear_all_layers(&mut path_state),
                    SwapLayers(layer1, layer2)                  => self.tes_swap_layers(layer1, layer2),

                    ClearSprite                                 => self.tes_clear_sprite(&mut path_state), 
                    Sprite(sprite_id)                           => self.tes_sprite(sprite_id), 
                    SpriteTransform(transform)                  => self.tes_sprite_transform(transform),
                    DrawSprite(sprite_id)                       => self.tes_draw_sprite(sprite_id),

                    Texture(texture_id, texture_op)             => self.tes_texture(texture_id, texture_op),
                    Gradient(gradient_id, gradient_op)          => self.tes_gradient(gradient_id, gradient_op),

                    // Fonts aren't directly rendered by the canvas renderer (use a helper to convert to textures or outlines)
                    Font(font_id, font_op)                      => self.tes_font(font_id, font_op),
                    DrawText(font_id, text, x, y)               => self.tes_draw_text(font_id, text, x, y),
                    BeginLineLayout(x, y, alignment)            => self.tes_begin_line_layout(x, y, alignment),
                    DrawLaidOutText                             => self.tes_draw_laid_out_text(),
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
            let mut textures                        = vec![];
            let mut actions_for_dynamic_textures    = HashMap::<render::TextureId, Vec<TextureRenderRequest>>::new();
            let viewport_size                       = self.viewport_size;

            // After performing the pending render instructions, the textures remain loaded until replaced
            for (_, render_request) in mem::take(&mut core.layer_textures).into_iter() {
                use self::TextureRenderRequest::*;
                match &render_request {
                    CreateBlankTexture(_, _, _) |
                    FromSprite(_, _, _)         |
                    CopyTexture(_, _)           => {
                        // These are always rendered
                        textures.push(render_request);
                    },

                    SetBytes(texture_id, _, _, _)   |
                    CreateMipMaps(texture_id)       |
                    Filter(texture_id, _)           => {
                        // These also attach to the actions if the target texture is a dynamic texture
                        if let Some(dynamic_actions) = actions_for_dynamic_textures.get_mut(texture_id) {
                            dynamic_actions.push(render_request.clone());
                        }

                        // These are always rendered
                        textures.push(render_request);
                    },

                    DynamicTexture(texture_id, layer_handle, _, _, _, _) => {
                        let texture_id      = *texture_id;
                        let current_state   = DynamicTextureState { viewport: viewport_size, sprite_modification_count: core.layer(*layer_handle).state.modification_count };

                        // Clear and start collecting any processing actions for this texture
                        actions_for_dynamic_textures.insert(texture_id, vec![]);

                        if core.dynamic_texture_state.get(&texture_id) != Some(&current_state) {
                            // These are rendered if the viewport or sprite has changed since the last time
                            textures.push(render_request.clone());

                            // Update the viewport data so this isn't re-rendered until it changes
                            core.dynamic_texture_state.insert(texture_id, current_state);
                        }

                        // Put back on the request list so we re-render this texture in the next frame
                        core.layer_textures.push((texture_id, render_request));
                    }
                }
            }

            // The layer_textures now contains the actions that need to be preserved for the next frame
            // This is mainly dynamic texture rendering, which needs to be amended with the post-processing actions that were applied
            for (_, render_request) in core.layer_textures.iter_mut() {
                use self::TextureRenderRequest::*;
                match render_request {
                    DynamicTexture(texture_id, _, _, _, _, post_processing) => { 
                        if let Some(actions) = actions_for_dynamic_textures.remove(texture_id) {
                            Arc::make_mut(post_processing).extend(actions);
                        }
                    }

                    _ => { /* Ignore */}
                }
            }

            // The list of texture actions is treated as a stack by the renderer stream, so reverse it
            textures.reverse();

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
