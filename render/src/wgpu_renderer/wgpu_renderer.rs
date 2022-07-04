use super::texture::*;
use super::shader_cache::*;
use super::wgpu_shader::*;
use super::render_target::*;
use super::renderer_state::*;
use super::pipeline_configuration::*;

use crate::action::*;
use crate::buffer::*;

use wgpu;
use wgpu::util;
use wgpu::util::{DeviceExt};

use std::mem;
use std::slice;
use std::ops::{Range};
use std::sync::*;
use std::collections::{HashMap};
use std::ffi::{c_void};

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

    /// The width of the target surface
    width: u32,

    /// The height of the target surface
    height: u32,

    /// The shaders that have been loaded for this renderer
    shaders: HashMap<WgpuShader, Arc<wgpu::ShaderModule>>,

    /// The vertex buffers for this renderer
    vertex_buffers: Vec<Option<wgpu::Buffer>>,

    /// The index buffers for this renderer
    index_buffers: Vec<Option<wgpu::Buffer>>,

    /// The textures for this renderer
    textures: Vec<Option<WgpuTexture>>,

    /// The render targets for this renderer
    render_targets: Vec<Option<RenderTarget>>,

    /// The cache of render pipeline states used by this renderer
    pipeline_states: HashMap<PipelineConfiguration, Arc<wgpu::RenderPipeline>>,

    /// The cache of shader modules that have been loaded for this render session
    shader_cache: ShaderCache<WgpuShader>,

    /// The currently selected render target
    active_render_target: Option<RenderTargetId>,
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
            shaders:                HashMap::new(),
            vertex_buffers:         vec![],
            index_buffers:          vec![],
            textures:               vec![],
            render_targets:         vec![],
            pipeline_states:        HashMap::new(),
            shader_cache:           ShaderCache::empty(device.clone()),
            active_render_target:   None,
            width:                  0,
            height:                 0,
        }
    }

    ///
    /// Sets up the surface to render at a new size
    ///
    pub fn prepare_to_render(&mut self, width: u32, height: u32) {
        let swapchain_format    = self.target_surface.get_supported_formats(&*self.adapter)[0];
        let surface_config      = wgpu::SurfaceConfiguration {
            usage:          wgpu::TextureUsages::RENDER_ATTACHMENT,
            format:         swapchain_format,
            width:          width,
            height:         height,
            present_mode:   wgpu::PresentMode::Fifo
        };

        self.target_surface.configure(&*self.device, &surface_config);

        self.width  = width;
        self.height = height;
    }

    ///
    /// Performs some rendering actions to this renderer's surface
    ///
    pub fn render_to_surface<Actions: IntoIterator<Item=RenderAction>>(&mut self, actions: Actions) {
        // Create the render state
        let mut render_state    = RendererState::new(Arc::clone(&self.queue), &*self.device);

        // Select the most recent render target, or the main frame buffer if the active render target is not set or does not exist
        self.select_main_frame_buffer(&mut render_state);

        if let Some(active_render_target) = self.active_render_target {
            self.select_render_target(active_render_target, &mut render_state);
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
                ShowFrameBuffer                                                                 => { /* This doesn't double-buffer so nothing to do */ }
                CreateTextureBgra(texture_id, Size2D(width, height))                            => { self.create_bgra_texture(texture_id, width, height); }
                CreateTextureMono(texture_id, Size2D(width, height))                            => { self.create_mono_texture(texture_id, width, height); }
                Create1DTextureBgra(texture_id, Size1D(width))                                  => { self.create_bgra_1d_texture(texture_id, width); }
                Create1DTextureMono(texture_id, Size1D(width))                                  => { self.create_mono_1d_texture(texture_id, width); }
                WriteTextureData(texture_id, Position2D(x1, y1), Position2D(x2, y2), data)      => { self.write_texture_data_2d(texture_id, x1, y1, x2, y2, data); }
                WriteTexture1D(texture_id, Position1D(x1), Position1D(x2), data)                => { self.write_texture_data_1d(texture_id, x1, x2, data); }
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
    }

    ///
    /// Updates the render pipeline if necessary
    ///
    #[inline]
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
                        let mut temp_data       = PipelineDescriptorTempStorage::default();
                        let matrix_bind_layout  = render_state.pipeline_configuration.matrix_bind_group_layout();
                        let matrix_bind_layout  = device.create_bind_group_layout(&matrix_bind_layout);
                        let bind_layout         = [&matrix_bind_layout];
                        let pipeline_layout     = render_state.pipeline_configuration.pipeline_layout(&bind_layout);
                        let pipeline_layout     = device.create_pipeline_layout(&pipeline_layout);
                        let descriptor          = render_state.pipeline_configuration.render_pipeline_descriptor(shader_cache, &pipeline_layout, &mut temp_data);

                        let new_pipeline        = device.create_render_pipeline(&descriptor);

                        Arc::new(new_pipeline)
                    });

                // Store in the render pass resources and queue up a function to activate it (the render pass must borrow the pipeline and can't use any other kind of
                // reference, so we need this rather complicated arrangement to borrow it again later)
                let pipeline        = Arc::clone(pipeline);
                let pipeline_index  = render_state.render_pass_resources.pipelines.len();
                render_state.render_pass_resources.pipelines.push(pipeline);

                render_state.render_pass.push(Box::new(move |resources, render_pass| {
                    render_pass.set_pipeline(&resources.pipelines[pipeline_index])
                }));
            }
        }
    }
    
    ///
    /// Sets the transform to used with the following render instructions
    ///
    fn set_transform(&mut self, matrix: Matrix, render_state: &mut RendererState) {
        render_state.write_matrix(&matrix);
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

        self.vertex_buffers[vertex_id] = Some(vertex_buffer);
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

        self.index_buffers[index_id] = Some(index_buffer);
    }
    
    ///
    /// Indicates that a vertex buffer is unused
    ///
    fn free_vertex_buffer(&mut self, VertexBufferId(vertex_id): VertexBufferId) {
        if let Some(Some(buffer)) = self.vertex_buffers.get(vertex_id) {
            buffer.destroy();
            self.vertex_buffers[vertex_id] = None;
        }
    }
    
    ///
    /// Indicates that an index buffer is unused
    ///
    fn free_index_buffer(&mut self, IndexBufferId(index_id): IndexBufferId) {
        if let Some(Some(buffer)) = self.index_buffers.get(index_id) {
            buffer.destroy();
            self.index_buffers[index_id] = None;
        }
    }
    
    ///
    /// Sets the blend mode for the following render instructions
    ///
    fn blend_mode(&mut self, blend_mode: BlendMode, state: &mut RendererState) {
        state.pipeline_configuration.blending_mode  = blend_mode;
        state.pipeline_config_changed               = true;
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
        let new_texture                 = WgpuTexture {
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
            self.active_render_target = Some(RenderTargetId(render_id));

            // Render to the existing render target
            state.run_render_pass();

            // Switch to rendering to this render target
            let texture         = new_render_target.texture();
            let texture_format  = new_render_target.texture_format();
            let texture_view    = texture.create_view(&wgpu::TextureViewDescriptor::default());

            state.render_pass_resources.target_view     = Some(Arc::new(texture_view));
            state.render_pass_resources.target_texture  = Some(texture);
            state.pipeline_configuration.texture_format = texture_format;
            state.pipeline_config_changed               = true;
        }
    }
    
    ///
    /// Renders to the main frame buffer
    ///
    fn select_main_frame_buffer(&mut self, state: &mut RendererState) {
        self.active_render_target = None;

        // Finish the current render pass
        state.run_render_pass();

        // Switch to the surface texture
        let surface_texture     = self.target_surface.get_current_texture().unwrap();
        let swapchain_format    = self.target_surface.get_supported_formats(&self.adapter)[0];
        let texture_view        = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());

        state.render_pass_resources.target_view     = Some(Arc::new(texture_view));
        state.render_pass_resources.target_texture  = None;
        state.pipeline_configuration.texture_format = swapchain_format;
        state.pipeline_config_changed               = true;
    }
    
    ///
    /// Blits a frame buffer to the current render target
    ///
    fn draw_frame_buffer(&mut self, RenderTargetId(source_buffer): RenderTargetId, region: FrameBufferRegion, alpha: f64, state: &mut RendererState) {

    }
    
    ///
    /// Creates a 2D texture with the BGRA pixel format
    ///
    fn create_bgra_texture(&mut self, TextureId(texture_id): TextureId, width: usize, height: usize) {

    }
    
    ///
    /// Creates a 2D monochrome texture
    ///
    fn create_mono_texture(&mut self, TextureId(texture_id): TextureId, width: usize, height: usize) {

    }
    
    ///
    /// Creates a 1D BGRA texture
    ///
    fn create_bgra_1d_texture(&mut self, TextureId(texture_id): TextureId, width: usize) {

    }
    
    ///
    /// Creates a 1D monochrome texture
    ///
    fn create_mono_1d_texture(&mut self, TextureId(texture_id): TextureId, width: usize) {

    }
    
    ///
    /// Writes byte data to a region of a 2D texture
    ///
    fn write_texture_data_2d(&mut self, TextureId(texture_id): TextureId, x1: usize, y1: usize, x2: usize, y2: usize, data: Arc<Vec<u8>>) {

    }
    
    ///
    /// Writes bytes data to a region of a 1D texture
    ///
    fn write_texture_data_1d(&mut self, TextureId(texture_id): TextureId, x1: usize, x2: usize, data: Arc<Vec<u8>>) {

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

    }
    
    ///
    /// Uses a particular shader for future rendering
    ///
    fn use_shader(&mut self, shader_type: ShaderType, state: &mut RendererState) {

    }
    
    ///
    /// Renders a set of triangles in a vertex buffer
    ///
    fn draw_triangles(&mut self, VertexBufferId(vertex_buffer_id): VertexBufferId, range: Range<usize>, state: &mut RendererState) {

    }
    
    ///
    /// Renders a set of triangles by looking up vertices referenced by an index buffer
    ///
    fn draw_indexed_triangles(&mut self, VertexBufferId(vertex_buffer_id): VertexBufferId, IndexBufferId(index_buffer_id): IndexBufferId, num_vertices: usize, state: &mut RendererState) {

    }
}
