use super::texture::*;
use super::pipeline::*;
use super::wgpu_shader::*;
use super::shader_cache::*;
use super::render_target::*;
use super::renderer_state::*;
use super::texture_settings::*;
use super::pipeline_configuration::*;

use crate::action::*;
use crate::buffer::*;

use flo_canvas;

use wgpu;
use wgpu::util;
use wgpu::util::{DeviceExt};

use std::mem;
use std::slice;
use std::ops::{Range};
use std::sync::*;
use std::collections::{HashMap};
use std::ffi::{c_void};
use std::num::{NonZeroU32};

///
/// Renderer that uses the `wgpu` abstract library as a render target
///
pub struct WgpuRenderer {
    /// A reference to the adapter this will render to
    adapter: Arc<wgpu::Adapter>,

    /// A reference to the device that this will render to
    device: Arc<wgpu::Device>,

    /// The command queue for the device
    queue: Arc<wgpu::Queue>,

    /// The surface that this renderer will target
    target_surface: Arc<wgpu::Surface>,

    /// The surface texture that is being written to
    target_surface_texture: Option<wgpu::SurfaceTexture>,

    /// The format of the target surface
    target_format: Option<wgpu::TextureFormat>,

    /// The width of the target surface
    width: u32,

    /// The height of the target surface
    height: u32,

    /// The shaders that have been loaded for this renderer
    shaders: HashMap<WgpuShader, Arc<wgpu::ShaderModule>>,

    /// The vertex buffers for this renderer
    vertex_buffers: Vec<Option<Arc<wgpu::Buffer>>>,

    /// The index buffers for this renderer
    index_buffers: Vec<Option<Arc<wgpu::Buffer>>>,

    /// The textures for this renderer
    textures: Vec<Option<WgpuTexture>>,

    /// The render targets for this renderer
    render_targets: Vec<Option<RenderTarget>>,

    /// The cache of render pipeline states used by this renderer
    pipeline_states: HashMap<PipelineConfiguration, Arc<Pipeline>>,

    /// The cache of shader modules that have been loaded for this render session
    shader_cache: ShaderCache<WgpuShader>,

    /// The currently selected render target
    active_render_target: Option<RenderTargetId>,

    /// The currently selected shader
    active_shader: Option<ShaderType>,

    /// The currently active blend mode
    active_blend_mode: Option<BlendMode>,
}

impl WgpuRenderer {
    ///
    /// Creates a new WGPU renderer
    ///
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>, target_surface: Arc<wgpu::Surface>, target_adapter: Arc<wgpu::Adapter>) -> WgpuRenderer {
        WgpuRenderer {
            adapter:                target_adapter,
            device:                 device.clone(),
            queue:                  queue,
            target_surface:         target_surface,
            target_format:          None,
            target_surface_texture: None,
            shaders:                HashMap::new(),
            vertex_buffers:         vec![],
            index_buffers:          vec![],
            textures:               vec![],
            render_targets:         vec![],
            pipeline_states:        HashMap::new(),
            shader_cache:           ShaderCache::empty(device.clone()),
            width:                  0,
            height:                 0,
            active_render_target:   None,
            active_shader:          Some(ShaderType::Simple { clip_texture: None }),
            active_blend_mode:      Some(BlendMode::SourceOver),
        }
    }

    ///
    /// Sets up the surface to render at a new size
    ///
    pub fn prepare_to_render(&mut self, width: u32, height: u32) {
        // Leave the settings as-is if the width and height are the same
        if width == self.width && height == self.height && self.target_format.is_some() {
            return;
        }

        // Clear the existing surface view
        self.target_surface_texture = None;

        // Fetch the format
        let possible_formats    = self.target_surface.get_supported_formats(&*self.adapter);
        let actual_format       = possible_formats.iter().filter(|format| !format.describe().srgb).next().copied();
        let actual_format       = actual_format.unwrap_or(possible_formats[0]);

        let surface_config      = wgpu::SurfaceConfiguration {
            usage:          wgpu::TextureUsages::RENDER_ATTACHMENT,
            format:         actual_format,
            width:          width,
            height:         height,
            present_mode:   wgpu::PresentMode::AutoVsync,
        };

        self.target_surface.configure(&*self.device, &surface_config);

        self.width          = width;
        self.height         = height;
        self.target_format  = Some(actual_format);
    }

    ///
    /// Performs some rendering actions to this renderer's surface
    ///
    pub fn render_to_surface<Actions: IntoIterator<Item=RenderAction>>(&mut self, actions: Actions) {
        // Create the render state
        let mut render_state    = RendererState::new(Arc::clone(&self.queue), Arc::clone(&self.device));

        // Select the most recent render target, or the main frame buffer if the active render target is not set or does not exist
        self.select_main_frame_buffer(&mut render_state);

        if let Some(active_render_target) = self.active_render_target.take() {
            self.select_render_target(active_render_target, &mut render_state);
        }

        // Set up the shader type
        if let Some(blend_mode) = self.active_blend_mode.take() {
            self.blend_mode(blend_mode, &mut render_state);
        }
        if let Some(shader) = self.active_shader.take() {
            self.use_shader(shader, &mut render_state);
        }

        // Evaluate the actions
        for action in actions {
            use self::RenderAction::*;

            match action {
                SetTransform(matrix)                                                            => { self.set_transform(matrix, &mut render_state); }
                CreateVertex2DBuffer(id, vertices)                                              => { self.create_vertex_buffer_2d(id, vertices); }
                CreateIndexBuffer(id, indices)                                                  => { self.create_index_buffer(id, indices); }
                FreeVertexBuffer(id)                                                            => { self.free_vertex_buffer(id); }
                FreeIndexBuffer(id)                                                             => { self.free_index_buffer(id); }
                BlendMode(blend_mode)                                                           => { self.blend_mode(blend_mode, &mut render_state); }
                CreateRenderTarget(render_id, texture_id, Size2D(width, height), render_type)   => { self.create_render_target(render_id, texture_id, width, height, render_type); }
                FreeRenderTarget(render_id)                                                     => { self.free_render_target(render_id); }
                SelectRenderTarget(render_id)                                                   => { self.select_render_target(render_id, &mut render_state); }
                RenderToFrameBuffer                                                             => { self.select_main_frame_buffer(&mut render_state); }
                DrawFrameBuffer(render_id, region, Alpha(alpha))                                => { self.draw_frame_buffer(render_id, region, alpha, &mut render_state); }
                ShowFrameBuffer                                                                 => { self.show_frame_buffer(&mut render_state); }
                CreateTextureBgra(texture_id, Size2D(width, height))                            => { self.create_bgra_texture(texture_id, width, height); }
                CreateTextureMono(texture_id, Size2D(width, height))                            => { self.create_mono_texture(texture_id, width, height); }
                Create1DTextureBgra(texture_id, Size1D(width))                                  => { self.create_bgra_1d_texture(texture_id, width); }
                Create1DTextureMono(texture_id, Size1D(width))                                  => { self.create_mono_1d_texture(texture_id, width); }
                WriteTextureData(texture_id, Position2D(x1, y1), Position2D(x2, y2), data)      => { self.write_texture_data_2d(texture_id, x1, y1, x2, y2, data, &mut render_state); }
                WriteTexture1D(texture_id, Position1D(x1), Position1D(x2), data)                => { self.write_texture_data_1d(texture_id, x1, x2, data, &mut render_state); }
                CreateMipMaps(texture_id)                                                       => { self.create_mipmaps(texture_id, &mut render_state); }
                CopyTexture(src_texture, tgt_texture)                                           => { self.copy_texture(src_texture, tgt_texture, &mut render_state); }
                FilterTexture(texture, filter)                                                  => { self.filter_texture(texture, filter, &mut render_state); }
                FreeTexture(texture_id)                                                         => { self.free_texture(texture_id); }
                Clear(color)                                                                    => { self.clear(color, &mut render_state); }
                UseShader(shader_type)                                                          => { self.use_shader(shader_type, &mut render_state); }
                DrawTriangles(buffer_id, buffer_range)                                          => { self.draw_triangles(buffer_id, buffer_range, &mut render_state); }
                DrawIndexedTriangles(vertex_buffer, index_buffer, num_vertices)                 => { self.draw_indexed_triangles(vertex_buffer, index_buffer, num_vertices, &mut render_state); }
            }
        }

        // Finish any pending render pass in the state
        render_state.run_render_pass();

        // Submit the queue
        self.queue.submit(Some(render_state.encoder.finish()));
    }

    ///
    /// Updates the render pipeline if necessary
    ///
    fn update_pipeline_if_needed(&mut self, render_state: &mut RendererState) {
        if render_state.pipeline_config_changed {
            // Reset the flag
            render_state.pipeline_config_changed = false;

            // If the pipeline config is actually different
            if Some(&render_state.pipeline_configuration) != render_state.active_pipeline_configuration.as_ref() {
                // Update the pipeline configuration
                render_state.active_pipeline_configuration = Some(render_state.pipeline_configuration.clone());

                // Borrow bits of the renderer we'll need later (so Rust doesn't complain about borrowing self again)
                let device          = &self.device;
                let shader_cache    = &mut self.shader_cache;  
                let pipeline_states = &mut self.pipeline_states;

                // Retrieve the pipeline from the cache or generate a new one
                let pipeline = pipeline_states.entry(render_state.pipeline_configuration.clone())
                    .or_insert_with(|| {
                        // Create the pipeline if we don't have one matching the configuration already
                        Arc::new(Pipeline::from_configuration(&render_state.pipeline_configuration, device, shader_cache))
                    });

                render_state.pipeline = Some(Arc::clone(pipeline));

                // Store the pipeline itself in the resources
                let render_pipeline = Arc::clone(&pipeline.pipeline);
                let pipeline_index  = render_state.render_pass_resources.pipelines.len();
                render_state.render_pass_resources.pipelines.push(render_pipeline);

                // Add a callback function to actually set up the render pipeline (we have to do it indirectly later on because it borrows its resources)
                render_state.render_pass.push(Box::new(move |resources, render_pass| {
                    // Set the pipeline
                    render_pass.set_pipeline(&resources.pipelines[pipeline_index]);
                }));

                // Rebind everything else
                render_state.pipeline_bindings_changed = true;
            }
        }

        // Refresh the bindings if they're marked as changed
        if render_state.pipeline_bindings_changed || render_state.pipeline_matrix_changed {
            render_state.pipeline_matrix_changed = false;

            render_state.bind_current_matrix();
        }

        if render_state.pipeline_bindings_changed {
            render_state.pipeline_bindings_changed = false;

            render_state.bind_current_clip_mask();
            render_state.bind_current_texture();
        }
    }
    
    ///
    /// Sets the transform to used with the following render instructions
    ///
    fn set_transform(&mut self, matrix: Matrix, render_state: &mut RendererState) {
        // Update the render buffer
        render_state.write_matrix(&matrix);

        // Set the matrix as ready to update at the next update_pipeline_if_needed call
        render_state.pipeline_matrix_changed = true;
    }
    
    ///
    /// Loads a buffer of vertex data to the GPU
    ///
    fn create_vertex_buffer_2d(&mut self, VertexBufferId(vertex_id): VertexBufferId, vertices: Vec<Vertex2D>) {
        // If there's an existing buffer with this index, drop it
        if let Some(Some(buffer)) = self.vertex_buffers.get(vertex_id) {
            buffer.destroy();
            self.vertex_buffers[vertex_id] = None;
        }

        // Convert the vertex buffer to a &[u8]
        let contents_void   = vertices.as_ptr() as *const c_void;
        let contents_len    = vertices.len() * mem::size_of::<Vertex2D>();
        let contents_u8     = unsafe { slice::from_raw_parts(contents_void as *const u8, contents_len) };

        // Create a new buffer on the device
        let vertex_buffer = self.device.create_buffer_init(&util::BufferInitDescriptor {
            label:      Some("create_vertex_buffer_2d"),
            contents:   contents_u8,
            usage:      wgpu::BufferUsages::VERTEX,
        });

        // Store associated with the vertex ID
        if vertex_id >= self.vertex_buffers.len() {
            self.vertex_buffers.extend((self.vertex_buffers.len()..(vertex_id+1))
                .into_iter()
                .map(|_| None));
        }

        self.vertex_buffers[vertex_id] = Some(Arc::new(vertex_buffer));
    }
    
    ///
    /// Loads a buffer of index data to the GPU
    ///
    fn create_index_buffer(&mut self, IndexBufferId(index_id): IndexBufferId, indices: Vec<u16>) {
        // If there's an existing buffer with this index, drop it
        if let Some(Some(buffer)) = self.index_buffers.get(index_id) {
            buffer.destroy();
            self.index_buffers[index_id] = None;
        }

        // Convert the index buffer to a &[u8]
        let contents_void   = indices.as_ptr() as *const c_void;
        let contents_len    = indices.len() * mem::size_of::<u16>();
        let contents_u8     = unsafe { slice::from_raw_parts(contents_void as *const u8, contents_len) };

        // Create a new buffer on the device
        let index_buffer = self.device.create_buffer_init(&util::BufferInitDescriptor {
            label:      Some("create_index_buffer_2d"),
            contents:   contents_u8,
            usage:      wgpu::BufferUsages::INDEX,
        });

        // Store associated with the index ID
        if index_id >= self.index_buffers.len() {
            self.index_buffers.extend((self.index_buffers.len()..(index_id+1))
                .into_iter()
                .map(|_| None));
        }

        self.index_buffers[index_id] = Some(Arc::new(index_buffer));
    }
    
    ///
    /// Indicates that a vertex buffer is unused
    ///
    fn free_vertex_buffer(&mut self, VertexBufferId(vertex_id): VertexBufferId) {
        if let Some(Some(buffer)) = self.vertex_buffers.get(vertex_id) {
            self.vertex_buffers[vertex_id] = None;
        }
    }
    
    ///
    /// Indicates that an index buffer is unused
    ///
    fn free_index_buffer(&mut self, IndexBufferId(index_id): IndexBufferId) {
        if let Some(Some(buffer)) = self.index_buffers.get(index_id) {
            self.index_buffers[index_id] = None;
        }
    }
    
    ///
    /// Sets the blend mode for the following render instructions
    ///
    fn blend_mode(&mut self, blend_mode: BlendMode, state: &mut RendererState) {
        if self.active_blend_mode == Some(blend_mode) {
            return;
        }

        self.active_blend_mode = Some(blend_mode);
        self.update_shader(self.active_shader, self.active_blend_mode, state);
    }
    
    ///
    /// Creates an off-screen render target and its texture
    ///
    fn create_render_target(&mut self, RenderTargetId(render_id): RenderTargetId, TextureId(texture_id): TextureId, width: usize, height: usize, render_target_type: RenderTargetType) {
        // Delete the old render target if it exists
        if let Some(old_render_target) = self.render_targets.get_mut(render_id) {
            *old_render_target = None;
        }

        if let Some(old_texture) = self.textures.get_mut(texture_id) {
            *old_texture = None;
        }

        // Create a new render target
        let new_render_target = RenderTarget::new(&*self.device, width as _, height as _, render_target_type);

        // Make space for the render target and the texture
        if render_id >= self.render_targets.len() {
            self.render_targets.extend((self.render_targets.len()..(render_id+1))
                .into_iter()
                .map(|_| None));
        }

        if texture_id >= self.textures.len() {
            self.textures.extend((self.textures.len()..(texture_id+1))
                .into_iter()
                .map(|_| None));
        }

        // Store the render target and texture. Render target textures have pre-multiplied alphas
        let new_texture = WgpuTexture {
            descriptor:         new_render_target.texture_descriptor(),
            texture:            new_render_target.texture(),
            is_premultiplied:   true,
        };

        self.textures[texture_id]       = Some(new_texture);
        self.render_targets[render_id]  = Some(new_render_target);
    }
    
    ///
    /// Releases a render target
    ///
    fn free_render_target(&mut self, RenderTargetId(render_id): RenderTargetId) {
        if let Some(old_render_target) = self.render_targets.get_mut(render_id) {
            *old_render_target = None;
        }
    }
    
    ///
    /// Picks a render target to use
    ///
    fn select_render_target(&mut self, RenderTargetId(render_id): RenderTargetId, state: &mut RendererState) {
        if let Some(Some(new_render_target)) = self.render_targets.get(render_id) {
            // Do nothing if the render target is already selected
            if Some(RenderTargetId(render_id)) == self.active_render_target {
                return;
            }

            self.active_render_target = Some(RenderTargetId(render_id));

            // Render to the existing render target
            state.run_render_pass();

            // Switch to rendering to this render target
            let texture         = new_render_target.texture();
            let target_size     = new_render_target.size();
            let texture_format  = new_render_target.texture_format();
            let samples         = new_render_target.sample_count();
            let texture_view    = texture.create_view(&wgpu::TextureViewDescriptor::default());

            state.target_size                                   = target_size;
            state.render_pass_resources.target_view             = Some(Arc::new(texture_view));
            state.render_pass_resources.target_texture          = Some(texture);
            state.pipeline_configuration.texture_format         = texture_format;
            state.pipeline_configuration.multisampling_count    = samples;
            state.pipeline_config_changed                       = true;

            self.update_pipeline_if_needed(state);
        }
    }
    
    ///
    /// Renders to the main frame buffer
    ///
    fn select_main_frame_buffer(&mut self, state: &mut RendererState) {
        self.active_render_target = None;

        // Ensure that there's a main frame buffer to render to
        if self.target_surface_texture.is_none() {
            let surface_texture = self.target_surface.get_current_texture().unwrap();
            self.target_surface_texture = Some(surface_texture);
        }

        // Finish the current render pass
        state.run_render_pass();

        // Switch to the surface texture
        let swapchain_format    = self.target_surface.get_supported_formats(&self.adapter)[0];
        let surface_texture     = self.target_surface_texture.as_ref().unwrap();
        let texture_view        = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());

        state.target_size                                   = (self.width, self.height);
        state.render_pass_resources.target_view             = Some(Arc::new(texture_view));
        state.render_pass_resources.target_texture          = None;
        state.pipeline_configuration.texture_format         = self.target_format.expect("prepare_to_render must be called before rendering");
        state.pipeline_configuration.multisampling_count    = None;
        state.pipeline_config_changed                       = true;

        self.update_pipeline_if_needed(state);
    }
    
    ///
    /// Blits a frame buffer to the current render target
    ///
    fn draw_frame_buffer(&mut self, RenderTargetId(source_buffer): RenderTargetId, region: FrameBufferRegion, alpha: f64, state: &mut RendererState) {
        // Fetch the corresponding render target
        let render_target = if let Some(Some(render_target)) = self.render_targets.get(source_buffer) { 
            render_target
        } else {
            return;
        };

        // Read the information from the render target
        let texture         = render_target.texture();
        let samples         = render_target.sample_count();
        let source_size     = render_target.size();
        let source_width    = source_size.0 as f32;
        let source_height   = source_size.1 as f32;

        // Copy the values we're going to update
        let old_texture             = state.input_texture.take();
        let old_matrix              = state.active_matrix;
        let old_texture_settings    = state.texture_settings;
        let old_pipeline_config     = state.pipeline_configuration.clone();

        // Configure for rendering the frame buffer
        let texture_type = if samples.is_none() { InputTextureType::Sampler } else { InputTextureType::Multisampled };

        state.input_texture                                     = Some(texture);
        state.pipeline_configuration.shader_module              = WgpuShader::Texture(StandardShaderVariant::NoClipping, texture_type, TexturePosition::Separate, AlphaBlendStep::Premultiply, ColorPostProcessingStep::NoPostProcessing);
        state.pipeline_configuration.blending_mode              = BlendMode::SourceOver;
        state.pipeline_configuration.source_is_premultiplied    = true;
        state.pipeline_config_changed                           = true;
        state.texture_settings                                  = TextureSettings { transform: Matrix::identity().0, alpha: alpha as _, ..Default::default() };

        // Work out a viewport matrix
        let target_size         = state.target_size;
        let target_width        = target_size.0 as f32;
        let target_height       = target_size.1 as f32;

        let scale_transform     = flo_canvas::Transform2D::scale(2.0/target_width, 2.0/target_height);
        let viewport_transform  = scale_transform * flo_canvas::Transform2D::translate(-(target_width/2.0), -(target_height/2.0));

        let viewport_matrix     = transform_to_matrix(&viewport_transform);
        state.write_matrix(&viewport_matrix);

        // Work out the region that's being rendered
        let min_x                       = region.min_x();
        let min_y                       = region.min_y();
        let max_x                       = region.max_x();
        let max_y                       = region.max_y();

        let min_x                       = (min_x + 1.0)/2.0;
        let min_y                       = (min_y + 1.0)/2.0;
        let max_x                       = (max_x + 1.0)/2.0;
        let max_y                       = (max_y + 1.0)/2.0;

        let min_x                       = min_x * source_width;
        let min_y                       = min_y * source_height;
        let max_x                       = max_x * source_width;
        let max_y                       = max_y * source_height;

        // Set up for rendering
        self.update_pipeline_if_needed(state);

        // Create the vertex buffer
        let triangles       = vec![
            Vertex2D::with_pos(min_x, min_y).with_texture_coordinates(min_x/source_width, (source_height-min_y)/source_height),
            Vertex2D::with_pos(min_x, max_y).with_texture_coordinates(min_x/source_width, (source_height-max_y)/source_height),
            Vertex2D::with_pos(max_x, min_y).with_texture_coordinates(max_x/source_width, (source_height-min_y)/source_height),

            Vertex2D::with_pos(max_x, min_y).with_texture_coordinates(max_x/source_width, (source_height-min_y)/source_height),
            Vertex2D::with_pos(max_x, max_y).with_texture_coordinates(max_x/source_width, (source_height-max_y)/source_height),
            Vertex2D::with_pos(min_x, max_y).with_texture_coordinates(min_x/source_width, (source_height-max_y)/source_height),
        ];

        let contents_void   = triangles.as_ptr() as *const c_void;
        let contents_len    = triangles.len() * mem::size_of::<Vertex2D>();
        let contents_u8     = unsafe { slice::from_raw_parts(contents_void as *const u8, contents_len) };

        let vertex_buffer   = self.device.create_buffer_init(&util::BufferInitDescriptor {
            label:      Some("draw_frame_buffer"),
            contents:   contents_u8,
            usage:      wgpu::BufferUsages::VERTEX,
        });
        let vertex_buffer   = Arc::new(vertex_buffer);

        // Queue up the render operation
        let buffer_index    = state.render_pass_resources.buffers.len();
        state.render_pass_resources.buffers.push(vertex_buffer);

        state.render_pass.push(Box::new(move |resources, render_pass| {
            let vertex_size = mem::size_of::<Vertex2D>();
            let start_pos   = 0;
            let end_pos     = (6 * vertex_size) as u64;

            render_pass.set_vertex_buffer(0, resources.buffers[buffer_index].slice(start_pos..end_pos));
            render_pass.draw(0..6, 0..1);
        }));

        // Restore the render state
        state.input_texture             = old_texture;
        state.active_matrix             = old_matrix;
        state.texture_settings          = old_texture_settings;
        state.pipeline_configuration    = old_pipeline_config;
        state.pipeline_config_changed   = true;
        state.pipeline_bindings_changed = true;
    }

    ///
    /// Displays the current frame buffer to the screen
    ///
    fn show_frame_buffer(&mut self, render_state: &mut RendererState) {
        // Finish the current render pass
        render_state.run_render_pass();

        // Present the current frame buffer
        if let Some(surface_texture) = self.target_surface_texture.take() {
            surface_texture.present();
        }

        // Fetch a new frame buffer
        if self.target_surface_texture.is_none() {
            let surface_texture = self.target_surface.get_current_texture().unwrap();
            self.target_surface_texture = Some(surface_texture);

            if self.active_render_target.is_none() {
                let surface_texture     = self.target_surface_texture.as_ref().unwrap();
                let texture_view        = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());

                render_state.render_pass_resources.target_view     = Some(Arc::new(texture_view));
                render_state.render_pass_resources.target_texture  = None;
            }
        }
    }
    
    ///
    /// Creates a 2D texture with the BGRA pixel format
    ///
    fn create_bgra_texture(&mut self, TextureId(texture_id): TextureId, width: usize, height: usize) {
        // Free the old texture if there is one
        if let Some(old_texture) = self.textures.get_mut(texture_id) {
            *old_texture = None;
        }

        // Texture is COPY_DST so we can write to it
        let mut descriptor = wgpu::TextureDescriptor {
            label:  Some("render_target"),
            size:   wgpu::Extent3d {
                width:                  width as _,
                height:                 height as _,
                depth_or_array_layers:  1,
            },
            mip_level_count:    1,
            sample_count:       1,
            dimension:          wgpu::TextureDimension::D2,
            format:             wgpu::TextureFormat::Rgba8Unorm,
            usage:              wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        };

        // Create the texture
        let new_texture = self.device.create_texture(&descriptor);
        let new_texture = WgpuTexture {
            descriptor:         descriptor,
            texture:            Arc::new(new_texture),
            is_premultiplied:   false,
        };

        // Store the texture
        if texture_id >= self.textures.len() {
            self.textures.extend((self.textures.len()..(texture_id+1))
                .into_iter()
                .map(|_| None));
        }

        self.textures[texture_id] = Some(new_texture);
    }
    
    ///
    /// Creates a 2D monochrome texture
    ///
    fn create_mono_texture(&mut self, TextureId(texture_id): TextureId, width: usize, height: usize) {
        // Free the old texture if there is one
        if let Some(old_texture) = self.textures.get_mut(texture_id) {
            *old_texture = None;
        }

        // Texture is COPY_DST so we can write to it
        let mut descriptor = wgpu::TextureDescriptor {
            label:  Some("render_target"),
            size:   wgpu::Extent3d {
                width:                  width as _,
                height:                 height as _,
                depth_or_array_layers:  1,
            },
            mip_level_count:    1,
            sample_count:       1,
            dimension:          wgpu::TextureDimension::D2,
            format:             wgpu::TextureFormat::R8Unorm,
            usage:              wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        };

        // Create the texture
        let new_texture = self.device.create_texture(&descriptor);
        let new_texture = WgpuTexture {
            descriptor:         descriptor,
            texture:            Arc::new(new_texture),
            is_premultiplied:   false,
        };

        // Store the texture
        if texture_id >= self.textures.len() {
            self.textures.extend((self.textures.len()..(texture_id+1))
                .into_iter()
                .map(|_| None));
        }

        self.textures[texture_id] = Some(new_texture);
    }
    
    ///
    /// Creates a 1D BGRA texture
    ///
    fn create_bgra_1d_texture(&mut self, TextureId(texture_id): TextureId, width: usize) {
        // Free the old texture if there is one
        if let Some(old_texture) = self.textures.get_mut(texture_id) {
            *old_texture = None;
        }

        // Texture is COPY_DST so we can write to it
        let mut descriptor = wgpu::TextureDescriptor {
            label:  Some("render_target"),
            size:   wgpu::Extent3d {
                width:                  width as _,
                height:                 1,
                depth_or_array_layers:  1,
            },
            mip_level_count:    1,
            sample_count:       1,
            dimension:          wgpu::TextureDimension::D1,
            format:             wgpu::TextureFormat::Rgba8Unorm,
            usage:              wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        };

        // Create the texture
        let new_texture = self.device.create_texture(&descriptor);
        let new_texture = WgpuTexture {
            descriptor:         descriptor,
            texture:            Arc::new(new_texture),
            is_premultiplied:   false,
        };

        // Store the texture
        if texture_id >= self.textures.len() {
            self.textures.extend((self.textures.len()..(texture_id+1))
                .into_iter()
                .map(|_| None));
        }

        self.textures[texture_id] = Some(new_texture);
    }
    
    ///
    /// Creates a 1D monochrome texture
    ///
    fn create_mono_1d_texture(&mut self, TextureId(texture_id): TextureId, width: usize) {
        // Free the old texture if there is one
        if let Some(old_texture) = self.textures.get_mut(texture_id) {
            *old_texture = None;
        }

        // Texture is COPY_DST so we can write to it
        let mut descriptor = wgpu::TextureDescriptor {
            label:  Some("render_target"),
            size:   wgpu::Extent3d {
                width:                  width as _,
                height:                 1,
                depth_or_array_layers:  1,
            },
            mip_level_count:    1,
            sample_count:       1,
            dimension:          wgpu::TextureDimension::D1,
            format:             wgpu::TextureFormat::R8Unorm,
            usage:              wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        };

        // Create the texture
        let new_texture = self.device.create_texture(&descriptor);
        let new_texture = WgpuTexture {
            descriptor:         descriptor,
            texture:            Arc::new(new_texture),
            is_premultiplied:   false,
        };

        // Store the texture
        if texture_id >= self.textures.len() {
            self.textures.extend((self.textures.len()..(texture_id+1))
                .into_iter()
                .map(|_| None));
        }

        self.textures[texture_id] = Some(new_texture);
    }
    
    ///
    /// Writes byte data to a region of a 2D texture
    ///
    fn write_texture_data_2d(&mut self, TextureId(texture_id): TextureId, x1: usize, y1: usize, x2: usize, y2: usize, data: Arc<Vec<u8>>, state: &mut RendererState) {
        if let Some(Some(texture)) = self.textures.get(texture_id) {
            state.run_render_pass();

            let (x1, x2)        = if x1 > x2 { (x2, x1) } else { (x1, x2) };
            let (y1, y2)        = if y1 > y2 { (y2, y1) } else { (y1, y2) };

            let bytes_per_pixel = texture.descriptor.format.describe().block_size as u64;

            let line_offset     = (y1 as u64) * (texture.descriptor.size.width as u64) * bytes_per_pixel;
            let pixel_offset    = (x1 as u64) * bytes_per_pixel;
            let bytes_per_row   = (texture.descriptor.size.width as u64) * bytes_per_pixel;

            let layout          = wgpu::ImageDataLayout {
                offset:         line_offset + pixel_offset,
                bytes_per_row:  Some(NonZeroU32::new(bytes_per_row as u32).unwrap()),
                rows_per_image: None,
            };

            self.queue.write_texture(texture.texture.as_image_copy(), &*data, layout, wgpu::Extent3d { width: (x2-x1) as u32, height: (y2-y1) as u32, depth_or_array_layers: 1 });
            state.render_pass_resources.textures.push(Arc::clone(&texture.texture));
        }
    }
    
    ///
    /// Writes bytes data to a region of a 1D texture
    ///
    fn write_texture_data_1d(&mut self, TextureId(texture_id): TextureId, x1: usize, x2: usize, data: Arc<Vec<u8>>, state: &mut RendererState) {
        if let Some(Some(texture)) = self.textures.get(texture_id) {
            let bytes_per_pixel = texture.descriptor.format.describe().block_size as u64;
            let layout          = wgpu::ImageDataLayout {
                offset:         (x1 as u64) * bytes_per_pixel,
                bytes_per_row:  Some(NonZeroU32::new(((texture.descriptor.size.width as u64) * bytes_per_pixel) as u32).unwrap()),
                rows_per_image: None,
            };

            self.queue.write_texture(texture.texture.as_image_copy(), &*data, layout, wgpu::Extent3d { width: (x2-x1) as u32, height: 1, depth_or_array_layers: 1 });
            state.render_pass_resources.textures.push(Arc::clone(&texture.texture));
        }
    }
    
    ///
    /// Generates the mipmap textures for a particular texture
    ///
    fn create_mipmaps(&mut self, TextureId(texture_id): TextureId, state: &mut RendererState) {

    }
    
    ///
    /// Creates a copy of a texture with a new ID
    ///
    fn copy_texture(&mut self, TextureId(src_texture_id): TextureId, TextureId(tgt_texture_id): TextureId, state: &mut RendererState) {
        if let Some(Some(src_texture)) = self.textures.get(src_texture_id) {
            // Copy the texture details
            let src_texture = src_texture.clone();

            // Clear out the destination texture if it already exists
            if tgt_texture_id >= self.textures.len() {
                self.textures.extend((self.textures.len()..(tgt_texture_id+1))
                    .into_iter()
                    .map(|_| None));
            }

            self.textures[tgt_texture_id] = None;

            // Create a new texture using the same definition as the original texture
            let new_texture_descriptor  = src_texture.descriptor.clone();
            let new_texture             = self.device.create_texture(&new_texture_descriptor);
            let new_texture             = WgpuTexture {
                descriptor:         new_texture_descriptor,
                texture:            Arc::new(new_texture),
                is_premultiplied:   src_texture.is_premultiplied,
            };

            // Finish the render pass (especially for the case where the source texture is also the render target)
            state.run_render_pass();

            // Copy the source texture to the target texture
            state.encoder.copy_texture_to_texture(src_texture.texture.as_image_copy(), new_texture.texture.as_image_copy(), src_texture.descriptor.size);

            // Store the new texture
            self.textures[tgt_texture_id] = Some(new_texture);
        }
    }
    
    ///
    /// Applies a filter effect to the content of a texture
    ///
    fn filter_texture(&mut self, TextureId(texture_id): TextureId, filter: Vec<TextureFilter>, state: &mut RendererState) {

    }
    
    ///
    /// Releases the data associated with a texture
    ///
    fn free_texture(&mut self, TextureId(texture_id): TextureId) {
        if let Some(old_texture) = self.textures.get_mut(texture_id) {
            *old_texture = None;
        }
    }
    
    ///
    /// Clears the current render target to a single colour
    ///
    fn clear(&mut self, color: Rgba8, state: &mut RendererState) {
        // Commit any existing rendering
        self.update_pipeline_if_needed(state);
        state.run_render_pass();

        // Set the clear color for the next render pass
        let Rgba8([r, g, b, a]) = color;

        let r       = (r as f64) / 255.0;
        let g       = (g as f64) / 255.0;
        let b       = (b as f64) / 255.0;
        let a       = (a as f64) / 255.0;
        let color   = wgpu::Color { r, g, b, a };
        
        state.render_pass_resources.clear = Some(color);
    }
    
    ///
    /// Uses a particular shader for future rendering
    ///
    fn use_shader(&mut self, shader_type: ShaderType, state: &mut RendererState) {
        if Some(shader_type) == self.active_shader {
            return;
        }

        self.active_shader = Some(shader_type);
        self.update_shader(self.active_shader, self.active_blend_mode, state);
    }

    ///
    /// Updates the render settings for a selected shader
    ///
    fn update_shader(&mut self, shader_type: Option<ShaderType>, blend_mode: Option<BlendMode>, state: &mut RendererState) {
        use self::ShaderType::*;

        let shader_type = if let Some(shader_type) = shader_type { shader_type } else { return; };
        let blend_mode  = if let Some(blend_mode) = blend_mode { blend_mode } else { return; };

        // Set the blend mode in the pipeline
        state.pipeline_configuration.blending_mode = blend_mode;

        // The post-processing step depends on the blend mode
        let post_processing = match blend_mode {
            BlendMode::Multiply     => ColorPostProcessingStep::InvertColorAlpha,
            BlendMode::Screen       => ColorPostProcessingStep::MultiplyAlpha,

            _                       => ColorPostProcessingStep::NoPostProcessing
        };

        // Set up the pipeline based on the shader type
        match shader_type {
            Simple { clip_texture } => {
                let clip_texture    = if let Some(TextureId(clip_texture)) = clip_texture {
                    if let Some(Some(texture)) = self.textures.get(clip_texture) {
                        Some(Arc::clone(&texture.texture))
                    } else {
                        None
                    }
                } else {
                    None
                };
                let variant         = if clip_texture.is_some() { StandardShaderVariant::ClippingMask } else { StandardShaderVariant::NoClipping };

                state.pipeline_configuration.shader_module              = WgpuShader::Simple(variant, post_processing);
                state.pipeline_configuration.source_is_premultiplied    = false;
            }

            DashedLine { clip_texture, .. } => {
                // TODO (this shader doesn't work anyway so should probably be deprecated)
            }

            Texture { texture, texture_transform, repeat, alpha, clip_texture } => {
                // Fetch the input texture
                let TextureId(texture_id)   = texture;
                let texture                 = if let Some(Some(texture)) = self.textures.get(texture_id) {
                    Some(texture)
                } else {
                    None
                };

                // Work out which clip texture to use (and the corresponding shader variant)
                let clip_texture    = if let Some(TextureId(clip_texture)) = clip_texture {
                    if let Some(Some(texture)) = self.textures.get(clip_texture) {
                        Some(Arc::clone(&texture.texture))
                    } else {
                        None
                    }
                } else {
                    None
                };
                let variant         = if clip_texture.is_some() { StandardShaderVariant::ClippingMask } else { StandardShaderVariant::NoClipping };

                // Alpha blend step depends on if the texture is pre-multiplied
                let alpha_blend = if let Some(true) = texture.map(|t| t.is_premultiplied) { 
                    AlphaBlendStep::Premultiply
                } else {
                    AlphaBlendStep::NoPremultiply
                };

                // See if the texture is multisampled or not
                let texture_type = match texture.map(|t| t.descriptor.sample_count) {
                    None    |
                    Some(0) |
                    Some(1) => InputTextureType::Sampler,
                    _       => InputTextureType::Multisampled,
                };

                // Set up the state
                state.texture_settings  = TextureSettings { transform: texture_transform.0, alpha: alpha as _, ..Default::default() };
                state.input_texture     = texture.map(|t| Arc::clone(&t.texture));

                if let Some(texture) = &texture {
                    state.pipeline_configuration.shader_module              = WgpuShader::Texture(variant, texture_type, TexturePosition::InputPosition, alpha_blend, post_processing);
                    state.pipeline_configuration.source_is_premultiplied    = texture.is_premultiplied;
                } else {
                    state.pipeline_configuration.shader_module              = WgpuShader::Simple(variant, post_processing);
                    state.pipeline_configuration.source_is_premultiplied    = false;
                }
            }

            LinearGradient { texture, texture_transform, repeat, alpha, clip_texture } => {

            }
        }

        // Mark the pipeline configuration as changed
        state.pipeline_config_changed   = true;
        state.pipeline_bindings_changed = true;
    }
    
    ///
    /// Renders a set of triangles in a vertex buffer
    ///
    fn draw_triangles(&mut self, VertexBufferId(vertex_buffer_id): VertexBufferId, range: Range<usize>, state: &mut RendererState) {
        if let Some(Some(buffer)) = self.vertex_buffers.get(vertex_buffer_id) {
            let buffer          = Arc::clone(&buffer);

            // Make sure that the pipeline is up to date
            self.update_pipeline_if_needed(state);

            // Add the buffer to the render pass resources
            let buffer_index    = state.render_pass_resources.buffers.len();
            state.render_pass_resources.buffers.push(buffer);

            // Set up a vertex buffer and draw the triangles during the render pass
            state.render_pass.push(Box::new(move |resources, render_pass| {
                let vertex_size = mem::size_of::<Vertex2D>();
                let start_pos   = (range.start * vertex_size) as u64;
                let end_pos     = (range.end * vertex_size) as u64;

                render_pass.set_vertex_buffer(0, resources.buffers[buffer_index].slice(start_pos..end_pos));
                render_pass.draw(0..range.len() as u32, 0..1);
            }));
        }
    }
    
    ///
    /// Renders a set of triangles by looking up vertices referenced by an index buffer
    ///
    fn draw_indexed_triangles(&mut self, VertexBufferId(vertex_buffer_id): VertexBufferId, IndexBufferId(index_buffer_id): IndexBufferId, num_vertices: usize, state: &mut RendererState) {
        if let (Some(Some(vertex_buffer)), Some(Some(index_buffer))) = (self.vertex_buffers.get(vertex_buffer_id), self.index_buffers.get(index_buffer_id)) {
            let vertex_buffer       = Arc::clone(vertex_buffer);
            let index_buffer        = Arc::clone(index_buffer);

            // Make sure that the pipeline is up to date
            self.update_pipeline_if_needed(state);

            // Add the buffers to the render pass resources
            let vertex_buffer_index = state.render_pass_resources.buffers.len();
            let index_buffer_index  = state.render_pass_resources.buffers.len()+1;

            state.render_pass_resources.buffers.push(vertex_buffer);
            state.render_pass_resources.buffers.push(index_buffer);

            // Set up a vertex buffer and draw the triangles during the render pass
            state.render_pass.push(Box::new(move |resources, render_pass| {
                render_pass.set_vertex_buffer(0, resources.buffers[vertex_buffer_index].slice(..));
                render_pass.set_index_buffer(resources.buffers[index_buffer_index].slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..num_vertices as u32, 0, 0..1);
            }));
        }
    }
}

///
/// Converts a canvas transform to a rendering matrix
///
fn transform_to_matrix(transform: &flo_canvas::Transform2D) -> Matrix {
    let flo_canvas::Transform2D(t) = transform;

    Matrix([
        [t[0][0], t[0][1], 0.0, t[0][2]],
        [t[1][0], t[1][1], 0.0, t[1][2]],
        [t[2][0], t[2][1], 1.0, t[2][2]],
        [0.0,     0.0,     0.0, 1.0]
    ])
}
