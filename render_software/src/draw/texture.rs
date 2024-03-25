use super::canvas_drawing::*;
use super::drawing_state::*;
use super::dynamic_sprites::*;

use crate::filters::*;
use crate::pixel::*;
use crate::pixel_programs::*;
use crate::render::*;
use crate::scanplan::*;

use flo_canvas as canvas;

use std::sync::*;

// TODO: we store Texture as Arc<Texture> but we also tend to use Arc<> internally: do we need both?

///
/// Represents a texture in a drawing
///
#[derive(Clone)]
pub struct Texture {
    /// The pixels that make up the texture
    pub (super) pixels: TexturePixels,

    /// The alpha value to apply to the texture when drawing it
    pub (super) fill_alpha: f64,
}

///
/// The data stored as part of a texture
///
#[derive(Clone)]
pub enum TexturePixels {
    /// An empty texture with a width and a height
    Empty(usize, usize),

    /// A texture in Rgba format
    Rgba(Arc<RgbaTexture>),

    /// A texture in 16-bit linear format
    Linear(Arc<U16LinearTexture>),

    /// A texture prepared for rendering as a mipmap (with no RGBA original)
    MipMap(Arc<MipMap<Arc<U16LinearTexture>>>),

    /// A texture prepared for rendering as a mipmap (with RGBA original)
    MipMapWithOriginal(Arc<RgbaTexture>, Arc<MipMap<Arc<U16LinearTexture>>>),

    /// A dynamic sprite, which renders on demand
    DynamicSprite(Arc<Mutex<DynamicSprite>>),
}

impl Texture {
    ///
    /// Converts this texture to a mip-mapped texture
    ///
    pub fn make_mip_map(&mut self, gamma: f64) {
        match &self.pixels {
            TexturePixels::Empty(_, _) => { }

            TexturePixels::Rgba(rgba_texture) => {
                // Convert the texture to a U16 linear texture
                let u16_texture = U16LinearTexture::from_rgba(rgba_texture, gamma);
                let width       = u16_texture.width();
                let height      = u16_texture.height();
                let mipmaps     = MipMap::from_texture(Arc::new(u16_texture), |previous_level| previous_level.create_mipmap().map(|new_level| Arc::new(new_level)), width, height);

                // Change this texture to a mipmap
                self.pixels = TexturePixels::MipMapWithOriginal(Arc::clone(rgba_texture), Arc::new(mipmaps));
            }

            TexturePixels::Linear(u16_texture) => {
                // Generate mip-maps for this texture
                let u16_texture = Arc::clone(u16_texture);
                let width       = u16_texture.width();
                let height      = u16_texture.height();
                let mipmaps     = MipMap::from_texture(u16_texture, |previous_level| previous_level.create_mipmap().map(|new_level| Arc::new(new_level)), width, height);

                // Change this texture to a mipmap
                self.pixels = TexturePixels::MipMap(Arc::new(mipmaps));
            }

            TexturePixels::MipMapWithOriginal(_, _) |
            TexturePixels::MipMap(_)                => {
                // Already mipmapped
            },

            TexturePixels::DynamicSprite(_)         => {
                // Counts as already mipmapped
            }
        }
    }
}

impl<TPixel, const N: usize> CanvasDrawing<TPixel, N>
where
    TPixel: 'static + Send + Sync + Pixel<N>,
{
    ///
    /// Performs a texture operation on this canvas drawing
    ///
    #[inline]
    pub (crate) fn texture(&mut self, texture_id: canvas::TextureId, texture_op: canvas::TextureOp) {
        use canvas::TextureOp::*;

        match texture_op {
            Create(size, canvas::TextureFormat::Rgba)       => { self.texture_create_rgba(texture_id, size); },
            Free                                            => { self.texture_free(texture_id); },
            SetBytes(position, size, bytes)                 => { self.texture_set_bytes(texture_id, position, size, bytes); },
            SetFromSprite(sprite_id, bounds)                => { self.texture_set_from_sprite(texture_id, sprite_id, bounds); },
            CreateDynamicSprite(sprite_id, bounds, size)    => { self.texture_create_dynamic_sprite(texture_id, sprite_id, bounds, size); },
            FillTransparency(alpha)                         => { self.texture_fill_transparency(texture_id, alpha as f64); },
            Copy(target_texture)                            => { self.texture_copy(texture_id, target_texture); },
            Filter(filter)                                  => { self.texture_filter(texture_id, filter); }
        }
    }

    ///
    /// Releases the memory being used by a texture
    ///
    #[inline]
    pub (crate) fn texture_free(&mut self, texture_id: canvas::TextureId) {
        self.textures.remove(&(self.current_namespace, texture_id));
    }

    ///
    /// Creates a blank RGBA texture of a particular size
    ///
    #[inline]
    pub (crate) fn texture_create_rgba(&mut self, texture_id: canvas::TextureId, canvas::TextureSize(width, height): canvas::TextureSize) {
        let width   = width as usize;
        let height  = height as usize;

        // Texture is initially just empty
        let texture = Texture { pixels: TexturePixels::Empty(width, height), fill_alpha: 1.0 };

        // Store it, replacing any existing texture with this ID
        self.textures.insert((self.current_namespace, texture_id), texture);
    }

    ///
    /// Sets the bytes for a region of the texture
    ///
    #[inline]
    pub (crate) fn texture_set_bytes(&mut self, texture_id: canvas::TextureId, canvas::TexturePosition(x, y): canvas::TexturePosition, canvas::TextureSize(width, height): canvas::TextureSize, bytes: Arc<Vec<u8>>) {
        if let Some(texture) = self.textures.get_mut(&(self.current_namespace, texture_id)) {
            // The texture exists: prepare to write to it
            let x           = x as usize;
            let y           = y as usize;
            let width       = width as usize;
            let height      = height as usize;

            // How the bytes are written depend on the format of the texture
            match &mut texture.pixels {
                TexturePixels::Empty(texture_w, texture_h) => {
                    let texture_w   = *texture_w;
                    let texture_h   = *texture_h;
                    let mut rgba    = RgbaTexture::from_pixels(texture_w, texture_h, vec![0u8; texture_w * texture_h*4]);
                    rgba.set_bytes(x, y, width, height, &*bytes);

                    texture.pixels = TexturePixels::Rgba(Arc::new(rgba));
                }

                TexturePixels::Rgba(rgba) => {
                    let rgba = Arc::make_mut(rgba);
                    rgba.set_bytes(x, y, width, height, &*bytes);
                }

                TexturePixels::Linear(linear) => {
                    let mut rgba = RgbaTexture::from_linear_texture(&**linear, self.gamma);
                    rgba.set_bytes(x, y, width, height, &*bytes);

                    texture.pixels = TexturePixels::Rgba(Arc::new(rgba));
                }

                TexturePixels::MipMap(mipmap) => {
                    let mut rgba = RgbaTexture::from_linear_texture(mipmap.mip_level(0), self.gamma);
                    rgba.set_bytes(x, y, width, height, &*bytes);

                    texture.pixels = TexturePixels::Rgba(Arc::new(rgba));
                }

                TexturePixels::MipMapWithOriginal(rgba, _) => {
                    let rgba_bytes = Arc::make_mut(rgba);
                    rgba_bytes.set_bytes(x, y, width, height, &*bytes);

                    texture.pixels = TexturePixels::Rgba(Arc::clone(rgba));
                }

                TexturePixels::DynamicSprite(_) => {
                    // This has no effect
                }
            }
        }
    }

    ///
    /// Sets the brush to fill using the specified texture
    ///
    pub (crate) fn fill_texture(&mut self, texture_id: canvas::TextureId, x1: f32, y1: f32, x2: f32, y2: f32) {
        // Fetch the state from this object
        let textures        = &mut self.textures;
        let current_state   = &mut self.current_state;
        let data_cache      = &mut self.program_data_cache;

        // Transform the coordinates to screen coordinates
        let (x1, y1) = current_state.transform.transform_point(x1, y1);
        let (x2, y2) = current_state.transform.transform_point(x2, y2);

        if let Some(texture) = textures.get_mut(&(self.current_namespace, texture_id)) {
            // Texture exists
            texture.make_mip_map(self.gamma);
            let fill_alpha = texture.fill_alpha;

            match &texture.pixels {
                TexturePixels::Empty(_, _) => {
                    DrawingState::release_program(&mut current_state.fill_program, data_cache);
                    current_state.next_fill_brush = Brush::TransparentSolidColor(canvas::Color::Rgba(0.0, 0.0, 0.0, 0.0));
                }

                TexturePixels::Rgba(rgba_texture) => {
                    // We want to make a transformation that maps x1, y1 to 0,0 and x2, y2 to w, h
                    let w = rgba_texture.width() as f32;
                    let h = rgba_texture.height() as f32;

                    let transform = canvas::Transform2D::translate(-x1, -y1);
                    let transform = canvas::Transform2D::scale(1.0/(x2-x1), 1.0/(y2-y1)) * transform;
                    let transform = canvas::Transform2D::scale(w, h) * transform;

                    // Set as the brush state
                    DrawingState::release_program(&mut current_state.fill_program, data_cache);
                    current_state.next_fill_brush = Brush::TransparentTexture(fill_alpha, Arc::clone(rgba_texture), transform);
                },

                TexturePixels::Linear(linear_texture) => {
                    // We want to make a transformation that maps x1, y1 to 0,0 and x2, y2 to w, h
                    let w = linear_texture.width() as f32;
                    let h = linear_texture.height() as f32;

                    let transform = canvas::Transform2D::translate(-x1, -y1);
                    let transform = canvas::Transform2D::scale(1.0/(x2-x1), 1.0/(y2-y1)) * transform;
                    let transform = canvas::Transform2D::scale(w, h) * transform;

                    // Set as the brush state
                    DrawingState::release_program(&mut current_state.fill_program, data_cache);
                    current_state.next_fill_brush = Brush::TransparentLinearTexture(fill_alpha, Arc::clone(linear_texture), transform);
                },

                TexturePixels::MipMap(mipmap) | TexturePixels::MipMapWithOriginal(_, mipmap) => {
                    // We want to make a transformation that maps x1, y1 to 0,0 and x2, y2 to w, h
                    let w = mipmap.mip_level(0).width() as f32;
                    let h = mipmap.mip_level(0).height() as f32;

                    let transform = canvas::Transform2D::translate(-x1, -y1);
                    let transform = canvas::Transform2D::scale(1.0/(x2-x1), 1.0/(y2-y1)) * transform;
                    let transform = canvas::Transform2D::scale(w, h) * transform;

                    // Set as the brush state
                    DrawingState::release_program(&mut current_state.fill_program, data_cache);
                    current_state.next_fill_brush = Brush::TransparentMipMapTexture(fill_alpha, Arc::clone(mipmap), transform);
                }

                TexturePixels::DynamicSprite(dynamic) => {
                    // Retrieve/render the texture
                    let dynamic = Arc::clone(dynamic);
                    let dynamic_texture = {
                        let mut dynamic = dynamic.lock().unwrap();
                        dynamic.get_u16_texture(self)
                    };
            
                    // Reborrow
                    let current_state   = &mut self.current_state;
                    let data_cache      = &mut self.program_data_cache;

                    // We want to make a transformation that maps x1, y1 to 0,0 and x2, y2 to w, h
                    let w = dynamic_texture.width() as f32;
                    let h = dynamic_texture.height() as f32;

                    let transform = canvas::Transform2D::translate(-x1, -y1);
                    let transform = canvas::Transform2D::scale(1.0/(x2-x1), 1.0/(y2-y1)) * transform;
                    let transform = canvas::Transform2D::scale(w, h) * transform;

                    // Set as the brush state
                    DrawingState::release_program(&mut current_state.fill_program, data_cache);
                    current_state.next_fill_brush = Brush::TransparentLinearTexture(fill_alpha, dynamic_texture, transform);
                }
            }
        }
    }

    ///
    /// Creates a texture by rendering a region from the specified sprite bounds
    ///
    pub (crate) fn texture_set_from_sprite(&mut self, texture_id: canvas::TextureId, sprite_id: canvas::SpriteId, canvas::SpriteBounds(origin, size): canvas::SpriteBounds) {
        let current_namespace   = self.current_namespace;
        let textures            = &mut self.textures;
        let sprites             = &self.sprites;
        let layers              = &mut self.layers;

        // This has no effect if no texture is defined at this location
        let existing_texture = textures.get_mut(&(current_namespace, texture_id));
        let existing_texture = if let Some(existing_texture) = existing_texture { existing_texture } else { return; };

        let existing_alpha   = existing_texture.fill_alpha;

        // Start with the width & height of the existing texture
        let (width, height) = match &existing_texture.pixels {
            TexturePixels::Empty(w, h)                    => (*w, *h),
            TexturePixels::Rgba(rgba_texture)             => (rgba_texture.width(), rgba_texture.height()),
            TexturePixels::Linear(linear_texture)         => (linear_texture.width(), linear_texture.height()),
            TexturePixels::MipMap(mipmap)                 |
            TexturePixels::MipMapWithOriginal(_, mipmap)  => (mipmap.width(), mipmap.height()),
            TexturePixels::DynamicSprite(_)               => (1, 1),
        };

        // Drop the existing texture so we can replace it
        textures.remove(&(current_namespace, texture_id));

        // Fetch the sprite corresponding to the sprite ID
        let sprite_layer = sprites.get(&(current_namespace, sprite_id))
            .and_then(|layer_handle| layers.get_mut(layer_handle.0));

        let sprite_layer = if let Some(sprite_layer) = sprite_layer {
            sprite_layer
        } else {
            // Replace the texture with an empty texture if the sprite does not exist
            textures.insert((current_namespace, texture_id), Texture { pixels: TexturePixels::Empty(width, height), fill_alpha: existing_alpha });
            return;
        };

        // The sprite transform maps from the sprite coordinates to the range -1,1 to 1,1
        let sprite_transform = sprite_layer.last_transform;

        // Upper and lower bounds are the coordinates that are the bounds of the area to render to the texture
        let lower_bounds    = sprite_transform.transform_point(origin.0 as _, origin.1 as _);
        let upper_bounds    = sprite_transform.transform_point((origin.0 + size.0) as _, (origin.1 + size.1) as _);
        let bounds_w        = upper_bounds.0-lower_bounds.0;
        let bounds_h        = upper_bounds.1-lower_bounds.1;

        // Map the bounds to the texture pixels
        let to_texture_pixels = canvas::Transform2D::scale((width as f32)/bounds_w, (height as f32)/bounds_h) * canvas::Transform2D::translate(-lower_bounds.0, -lower_bounds.1);

        // Transform the edges from the layer to prepare them to render
        // TODO: could be better to use a transform in the renderer instead (which is what the canvas renderer does)
        sprite_layer.edges.prepare_to_render();
        let mut edges = sprite_layer.edges.transform(&to_texture_pixels);
        edges.prepare_to_render();

        // Create a background scan planner using the default pixel colour for the sprite
        // We need a background planner to clear the background colour
        let background_col  = SolidColorData(TPixel::default());
        let background_data = self.program_cache.program_cache.store_program_data(&self.program_cache.solid_color, &mut self.program_data_cache, background_col);
        let background      = BackgroundScanPlanner::new(ShardScanPlanner::default(), background_data);

        // Render the new texture
        let pixels = {
            // Use the scan planner to create a frame renderer
            let mut pixels      = vec![0u16; width*height*4];
            let renderer        = EdgePlanRegionRenderer::new(background, ScanlineRenderer::new(self.program_runner(height as _)));
            let frame_renderer  = U16LinearFrameRenderer::new(renderer);

            // Call the frame renderer to generate the pixels
            let region = FrameSize { width, height };
            frame_renderer.render(&region, &edges, &mut pixels);

            pixels
        };

        // Release the data we were using in the planner
        self.program_data_cache.release_program_data(background_data);

        // Save the pixels to the texture
        let texture = U16LinearTexture::from_pixels(width, height, pixels);
        self.textures.insert((current_namespace, texture_id), Texture { pixels: TexturePixels::Linear(Arc::new(texture)), fill_alpha: existing_alpha });
    }

    ///
    /// Creates a copy of a texture in another texture
    ///
    pub fn texture_copy(&mut self, source_texture: canvas::TextureId, target_texture: canvas::TextureId) {
        let current_namespace   = self.current_namespace;
        let textures            = &mut self.textures;
        let source_texture      = textures.get_mut(&(current_namespace, source_texture));

        if let Some(source_texture) = source_texture {
            // Clone the texture and add a copied texture
            let copied_texture = source_texture.clone();
            textures.insert((current_namespace, target_texture), copied_texture);
        } else {
            // Source texture doesn't exist: delete the target texture
            textures.remove(&(current_namespace, target_texture));
        }
    }

    ///
    /// Sets the transparency to use with a texture when rendering it
    ///
    pub fn texture_fill_transparency(&mut self, texture: canvas::TextureId, alpha: f64) {
        if let Some(texture) = self.textures.get_mut(&(self.current_namespace, texture)) {
            texture.fill_alpha = alpha;
        }
    }

    ///
    /// Applies a filter to a texture
    ///
    pub fn texture_filter(&mut self, texture_id: canvas::TextureId, filter: canvas::TextureFilter) {
        use canvas::TextureFilter::*;

        // TODO: for gaussian blur we can apply both filters at the same time (which is more efficient, but a bit more complicated to implement)
        match filter {
            GaussianBlur(radius)        => { self.texture_apply_filter(texture_id, HorizontalKernelFilter::with_gaussian_blur_radius(radius as _)); self.texture_apply_filter(texture_id, VerticalKernelFilter::with_gaussian_blur_radius(radius as _)); }
            AlphaBlend(alpha)           => { self.texture_apply_filter(texture_id, AlphaBlendFilter::with_alpha(alpha as _)); },

            Mask(mask_texture_id) => {
                let filter = self.texture_mask_filter(mask_texture_id, texture_id);

                self.texture_apply_filter(texture_id, filter); 
            },

            DisplacementMap(displacement_texture, x_offset, y_offset) => { 
                let filter = self.texture_displacement_filter(displacement_texture, texture_id, x_offset as _, y_offset as _);

                self.texture_apply_filter(texture_id, filter); 
            },
        }
    }

    ///
    /// Creates a mask filter from a texture
    ///
    pub fn texture_mask_filter(&mut self, mask_texture_id: canvas::TextureId, target_texture_id: canvas::TextureId) -> MaskFilter<TPixel, N> {
        // Fetch the size of the target texture
        let (texture_width, texture_height) = if let Some(texture) = self.textures.get(&(self.current_namespace, target_texture_id)) {
            match &texture.pixels {
                TexturePixels::Empty(w, h)                      => (*w, *h),
                TexturePixels::Rgba(rgba)                       => (rgba.width(), rgba.height()),
                TexturePixels::Linear(linear)                   => (linear.width(), linear.height()),
                TexturePixels::MipMap(mipmap)                   |
                TexturePixels::MipMapWithOriginal(_, mipmap)    => (mipmap.width(), mipmap.height()),
                TexturePixels::DynamicSprite(dynamic)           => {
                    let dynamic = Arc::clone(dynamic);
                    let texture = dynamic.lock().unwrap().get_u16_texture(self);

                    (texture.width(), texture.height())
                }
            }
        } else {
            (1, 1)
        };

        // Read the mask texture (we use a 1x1 empty texture if the texture is missing)
        let mask_texture = loop {
            let texture = self.textures.get(&(self.current_namespace, mask_texture_id));
            let texture = if let Some(texture) = texture { texture } else { break Arc::new(U16LinearTexture::from_pixels(1, 1, vec![0, 0, 0, 0])); };

            match &texture.pixels {
                TexturePixels::Empty(_, _) => {
                    break Arc::new(U16LinearTexture::from_pixels(1, 1, vec![0, 0, 0, 0]))
                }

                TexturePixels::Rgba(_) | TexturePixels::Linear(_) => {
                    // Convert to a mip-map so we can read as a U16 texture
                    self.textures.get_mut(&(self.current_namespace, mask_texture_id))
                        .unwrap().make_mip_map(self.gamma);                    
                }

                TexturePixels::MipMap(texture) | TexturePixels::MipMapWithOriginal(_, texture) => {
                    break Arc::clone(texture.mip_level(0));
                }

                TexturePixels::DynamicSprite(dynamic) => {
                    let dynamic = Arc::clone(dynamic);
                    break dynamic.lock().unwrap().get_u16_texture(self);
                }
            }
        };


        let (mask_width, mask_height) = (mask_texture.width(), mask_texture.height());
        let mult_x = mask_width as f64 / texture_width as f64;
        let mult_y = mask_height as f64 / texture_height as f64;

        MaskFilter::with_mask(&mask_texture, mult_x, mult_y)
    }

    ///
    /// Creates a displacement filter from a texture
    ///
    pub fn texture_displacement_filter(&mut self, displacement_texture_id: canvas::TextureId, target_texture_id: canvas::TextureId, x_offset: f64, y_offset: f64) -> DisplacementMapFilter<TPixel, N> {
        // Fetch the size of the target texture
        let (texture_width, texture_height) = if let Some(texture) = self.textures.get(&(self.current_namespace, target_texture_id)) {
            match &texture.pixels {
                TexturePixels::Empty(w, h)                      => (*w, *h),
                TexturePixels::Rgba(rgba)                       => (rgba.width(), rgba.height()),
                TexturePixels::Linear(linear)                   => (linear.width(), linear.height()),
                TexturePixels::MipMap(mipmap)                   |
                TexturePixels::MipMapWithOriginal(_, mipmap)    => (mipmap.width(), mipmap.height()),
                TexturePixels::DynamicSprite(dynamic)           => {
                    let dynamic = Arc::clone(dynamic);
                    let texture = dynamic.lock().unwrap().get_u16_texture(self);

                    (texture.width(), texture.height())
                }
            }
        } else {
            (1, 1)
        };

        // Read the displacement map texture (we use a 1x1 empty texture if the texture is missing)
        let displacement_texture = loop {
            let texture = self.textures.get(&(self.current_namespace, displacement_texture_id));
            let texture = if let Some(texture) = texture { texture } else { break Arc::new(U16LinearTexture::from_pixels(1, 1, vec![0, 0, 0, 0])); };

            match &texture.pixels {
                TexturePixels::Empty(_, _) => {
                    break Arc::new(U16LinearTexture::from_pixels(1, 1, vec![0, 0, 0, 0]))
                }

                TexturePixels::Rgba(_) | TexturePixels::Linear(_) => {
                    // Convert to a mip-map so we can read as a U16 texture
                    self.textures.get_mut(&(self.current_namespace, displacement_texture_id))
                        .unwrap().make_mip_map(self.gamma);                    
                }

                TexturePixels::MipMap(texture) | TexturePixels::MipMapWithOriginal(_, texture) => {
                    break Arc::clone(texture.mip_level(0));
                }

                TexturePixels::DynamicSprite(dynamic) => {
                    let dynamic = Arc::clone(dynamic);
                    break dynamic.lock().unwrap().get_u16_texture(self);
                }
            }
        };

        let (displ_width, displ_height) = (displacement_texture.width(), displacement_texture.height());
        let mult_x = displ_width as f64 / texture_width as f64;
        let mult_y = displ_height as f64 / texture_height as f64;

        // Create the filter from the texture
        DisplacementMapFilter::with_displacement_map(&displacement_texture, x_offset, y_offset, mult_x, mult_y, self.gamma)
    }

    ///
    /// Applies a filter to the texture with the specified ID (replacing the texture)
    ///
    pub fn texture_apply_filter(&mut self, texture_id: canvas::TextureId, filter: impl 'static + Send + Sync + PixelFilter<Pixel=TPixel>) {
        // TODO: it should be possible to filter the texture entirely in-place, as the filter will always be reading ahead of the pixels where we need to write to
        // this is much more memory efficient as it saves us allocating a whole new buffer for the filtered texture (but it's a bit tricky to wrangle in Rust as
        // we're writing to the same buffer that we're reading from)

        // Load the texture
        let texture = self.textures.get_mut(&(self.current_namespace, texture_id));

        // Replace the texture with a filtered version
        if let Some(texture) = texture {
            // Convert the texture, apply the filter and convert back to pixels
            let new_texture_pixels = match &texture.pixels {
                TexturePixels::Empty(width, height) => { 
                    // Assuming the filter has no effect on empty pixels
                    TexturePixels::Empty(*width, *height) 
                }

                TexturePixels::Rgba(rgba_texture) => {
                    // Convert to pixels on the fly
                    let width   = rgba_texture.width();
                    let height  = rgba_texture.height();

                    let new_pixels = filter_texture(&**rgba_texture, &filter);
                    texture_load_from_pixels(new_pixels, width, height)
                }

                TexturePixels::Linear(linear_texture) => {
                    let width   = linear_texture.width();
                    let height  = linear_texture.height();

                    // Convert to pixels on the fly
                    let new_pixels = filter_texture(&**linear_texture, &filter);
                    texture_load_from_pixels(new_pixels, width, height)
                }

                TexturePixels::MipMap(mipmap) |
                TexturePixels::MipMapWithOriginal(_, mipmap) => {
                    let width   = mipmap.width();
                    let height  = mipmap.height();

                    // Use the first mip level to do the filtering
                    let new_pixels = filter_texture(&**mipmap.mip_level(0), &filter);
                    texture_load_from_pixels(new_pixels, width, height)
                }

                TexturePixels::DynamicSprite(dynamic) => {
                    dynamic.lock().unwrap().apply_filter(filter);
                    return;
                }
            };

            // Store the new pixels
            texture.pixels = new_texture_pixels;
        }
    }
}

///
/// Creates a texture by loading from an iterator of pixels
///
pub fn texture_load_from_pixels<TPixel: Pixel<N>, const N: usize>(pixels: impl Iterator<Item=Vec<TPixel>>, width: usize, height: usize) -> TexturePixels {
    // Convert the pixels to u16 values
    let mut converted_pixels = vec![0u16; width * height * 4];

    for (ypos, pixel_line) in pixels.enumerate() {
        let start_pos   = width * ypos * 4;
        let end_pos     = start_pos + width * 4;

        let target_pixel = U16LinearPixel::u16_slice_as_linear_pixels(&mut converted_pixels[start_pos..end_pos]);
        TPixel::to_linear_colorspace(&pixel_line, target_pixel);
    }

    // Create the texture from the result
    let linear_texture = U16LinearTexture::from_pixels(width, height, converted_pixels);
    TexturePixels::Linear(Arc::new(linear_texture))
}
