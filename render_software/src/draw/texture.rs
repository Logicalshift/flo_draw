use super::canvas_drawing::*;
use super::drawing_state::*;
use super::renderer::*;

use crate::pixel::*;
use crate::render::*;
use crate::scanplan::*;

use flo_canvas as canvas;

use std::sync::*;

// TODO: we store Texture as Arc<Texture> but we also tend to use Arc<> internally: do we need both?

///
/// The data stored as part of a texture
///
#[derive(Clone)]
pub enum Texture {
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
}

impl Texture {
    ///
    /// Converts this texture to a mip-mapped texture
    ///
    pub fn make_mip_map(&mut self, gamma: f64) {
        match self {
            Texture::Empty(_, _) => { }

            Texture::Rgba(rgba_texture) => {
                // Convert the texture to a U16 linear texture
                let u16_texture = U16LinearTexture::from_rgba(rgba_texture, gamma);
                let width       = u16_texture.width();
                let height      = u16_texture.height();
                let mipmaps     = MipMap::from_texture(Arc::new(u16_texture), |previous_level| previous_level.create_mipmap().map(|new_level| Arc::new(new_level)), width, height);

                // Change this texture to a mipmap
                *self = Texture::MipMapWithOriginal(Arc::clone(rgba_texture), Arc::new(mipmaps));
            }

            Texture::Linear(u16_texture) => {
                // Generate mip-maps for this texture
                let u16_texture = Arc::clone(u16_texture);
                let width       = u16_texture.width();
                let height      = u16_texture.height();
                let mipmaps     = MipMap::from_texture(u16_texture, |previous_level| previous_level.create_mipmap().map(|new_level| Arc::new(new_level)), width, height);

                // Change this texture to a mipmap
                *self = Texture::MipMap(Arc::new(mipmaps));
            }

            Texture::MipMapWithOriginal(_, _) |
            Texture::MipMap(_)                => {
                // Already mipmapped
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
            CreateDynamicSprite(sprite_id, bounds, size)    => { /* todo!() */ },
            FillTransparency(alpha)                         => { /* todo!() */ },
            Copy(target_texture)                            => { /* todo!() */ },
            Filter(filter)                                  => { /* todo!() */ }
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
        let texture = Texture::Empty(width, height);

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
            match texture {
                Texture::Empty(texture_w, texture_h) => {
                    let texture_w   = *texture_w;
                    let texture_h   = *texture_h;
                    let mut rgba    = RgbaTexture::from_pixels(texture_w, texture_h, vec![0u8; texture_w * texture_h*4]);
                    rgba.set_bytes(x, y, width, height, &*bytes);

                    *texture = Texture::Rgba(Arc::new(rgba));
                }

                Texture::Rgba(rgba) => {
                    let rgba = Arc::make_mut(rgba);
                    rgba.set_bytes(x, y, width, height, &*bytes);
                }

                Texture::Linear(linear) => {
                    let mut rgba = RgbaTexture::from_linear_texture(&**linear, self.gamma);
                    rgba.set_bytes(x, y, width, height, &*bytes);

                    *texture = Texture::Rgba(Arc::new(rgba));
                }

                Texture::MipMap(mipmap) => {
                    let mut rgba = RgbaTexture::from_linear_texture(mipmap.mip_level(0), self.gamma);
                    rgba.set_bytes(x, y, width, height, &*bytes);

                    *texture = Texture::Rgba(Arc::new(rgba));
                }

                Texture::MipMapWithOriginal(rgba, _) => {
                    let rgba_bytes = Arc::make_mut(rgba);
                    rgba_bytes.set_bytes(x, y, width, height, &*bytes);

                    *texture = Texture::Rgba(Arc::clone(rgba));
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

        // Transform the coordiantes to screen coordinates
        let (x1, y1) = current_state.transform.transform_point(x1, y1);
        let (x2, y2) = current_state.transform.transform_point(x2, y2);

        if let Some(texture) = textures.get_mut(&(self.current_namespace, texture_id)) {
            // Texture exists
            texture.make_mip_map(self.gamma);

            match texture {
                Texture::Empty(_, _) => {
                    DrawingState::release_program(&mut current_state.fill_program, data_cache);
                    current_state.next_fill_brush = Brush::TransparentSolidColor(canvas::Color::Rgba(0.0, 0.0, 0.0, 0.0));
                }

                Texture::Rgba(rgba_texture) => {
                    // We want to make a transformation that maps x1, y1 to 0,0 and x2, y2 to w, h
                    let w = rgba_texture.width() as f32;
                    let h = rgba_texture.height() as f32;

                    let transform = canvas::Transform2D::translate(-x1, -y1);
                    let transform = canvas::Transform2D::scale(1.0/(x2-x1), 1.0/(y2-y1)) * transform;
                    let transform = canvas::Transform2D::scale(w, h) * transform;

                    // Set as the brush state
                    DrawingState::release_program(&mut current_state.fill_program, data_cache);
                    current_state.next_fill_brush = Brush::TransparentTexture(Arc::clone(rgba_texture), transform);
                },

                Texture::Linear(linear_texture) => {
                    // We want to make a transformation that maps x1, y1 to 0,0 and x2, y2 to w, h
                    let w = linear_texture.width() as f32;
                    let h = linear_texture.height() as f32;

                    let transform = canvas::Transform2D::translate(-x1, -y1);
                    let transform = canvas::Transform2D::scale(1.0/(x2-x1), 1.0/(y2-y1)) * transform;
                    let transform = canvas::Transform2D::scale(w, h) * transform;

                    // Set as the brush state
                    DrawingState::release_program(&mut current_state.fill_program, data_cache);
                    current_state.next_fill_brush = Brush::TransparentLinearTexture(Arc::clone(linear_texture), transform);
                },

                Texture::MipMap(mipmap) | Texture::MipMapWithOriginal(_, mipmap) => {
                    // We want to make a transformation that maps x1, y1 to 0,0 and x2, y2 to w, h
                    let w = mipmap.mip_level(0).width() as f32;
                    let h = mipmap.mip_level(0).height() as f32;

                    let transform = canvas::Transform2D::translate(-x1, -y1);
                    let transform = canvas::Transform2D::scale(1.0/(x2-x1), 1.0/(y2-y1)) * transform;
                    let transform = canvas::Transform2D::scale(w, h) * transform;

                    // Set as the brush state
                    DrawingState::release_program(&mut current_state.fill_program, data_cache);
                    current_state.next_fill_brush = Brush::TransparentMipMapTexture(Arc::clone(mipmap), transform);
                }
            }
        }
    }

    ///
    /// Creates a texture by rendering a region from the specified sprite bounds
    ///
    pub (crate) fn texture_set_from_sprite(&mut self, texture_id: canvas::TextureId, sprite_id: canvas::SpriteId, bounds: canvas::SpriteBounds) {
        let current_namespace   = self.current_namespace;
        let textures            = &mut self.textures;
        let sprites             = &self.sprites;
        let layers              = &self.layers;
        
        // This has no effect if no texture is defined at this location
        let existing_texture = textures.get_mut(&(current_namespace, texture_id));
        let existing_texture = if let Some(existing_texture) = existing_texture { existing_texture } else { return; };

        // Start with the width & height of the existing texture
        let (width, height) = match existing_texture {
            Texture::Empty(w, h)                    => (*w, *h),
            Texture::Rgba(rgba_texture)             => (rgba_texture.width(), rgba_texture.height()),
            Texture::Linear(linear_texture)         => (linear_texture.width(), linear_texture.height()),
            Texture::MipMap(mipmap)                 |
            Texture::MipMapWithOriginal(_, mipmap)  => (mipmap.width(), mipmap.height())
        };

        // Drop the existing texture so we can replace it
        textures.remove(&(current_namespace, texture_id));

        // Fetch the sprite corresponding to the sprite ID
        let sprite_layer = sprites.get(&(current_namespace, sprite_id))
            .and_then(|layer_handle| layers.get(layer_handle.0));

        let sprite_layer = if let Some(sprite_layer) = sprite_layer {
            sprite_layer
        } else {
            // Replace the texture with an empty texture if the sprite does not exist
            textures.insert((current_namespace, texture_id), Texture::Empty(width, height));
            return;
        };

        // The sprite transform maps from the sprite coordinates (which are also the bounds) to the 
        let sprite_transform = sprite_layer.last_transform;
        let sprite_transform = canvas::Transform2D::scale(128.0, 128.0) * canvas::Transform2D::translate(1.0, 1.0);

        // Transform the edges from the layer to prepare them to render
        // TODO: could be better to use a transform in the renderer instead (which is what the canvas renderer does)
        let mut edges = sprite_layer.edges.transform(&sprite_transform);
        edges.prepare_to_render();

        // Render the new texture (TODO: we need to take account of the bounds here)
        let pixels = {
            let mut pixels      = vec![0u16; width*height*4];
            let renderer        = EdgePlanRegionRenderer::new(ShardScanPlanner::default(), ScanlineRenderer::new(self.program_runner(height as _)));
            let frame_renderer  = U16LinearFrameRenderer::new(renderer);

            // The source is a sprite renderer
            let region = FrameSize { width, height };

            frame_renderer.render(&region, &edges, &mut pixels);
            pixels
        };

        // Save the pixels to the texture
        let texture = U16LinearTexture::from_pixels(width, height, pixels);
        self.textures.insert((current_namespace, texture_id), Texture::Linear(Arc::new(texture)));
    }
}
