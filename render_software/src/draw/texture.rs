use super::canvas_drawing::*;
use super::drawing_state::*;

use crate::pixel::*;

use flo_canvas as canvas;

use std::sync::*;

// TODO: we store Texture as Arc<Texture> but we also tend to use Arc<> internally: do we need both?

///
/// The data stored as part of a texture
///
#[derive(Clone)]
pub enum Texture {
    /// A texture in Rgba format
    Rgba(Arc<RgbaTexture>),

    /// A texture prepared for rendering as a mipmap
    MipMap(Arc<RgbaTexture>, Arc<MipMap<Arc<U16LinearTexture>>>),
}

impl Texture {
    ///
    /// Converts this texture to a mip-mapped texture
    ///
    pub fn make_mip_map(&mut self, gamma: f64) {
        match self {
            Texture::Rgba(rgba_texture) => {
                // Convert the texture to a U16 linear texture
                let u16_texture = U16LinearTexture::from_rgba(rgba_texture, gamma);
                let width       = u16_texture.width();
                let height      = u16_texture.height();
                let mipmaps     = MipMap::from_texture(Arc::new(u16_texture), |previous_level| previous_level.create_mipmap().map(|new_level| Arc::new(new_level)), width, height);

                // Change this texture to a mipmap
                *self = Texture::MipMap(Arc::clone(rgba_texture), Arc::new(mipmaps));
            }

            Texture::MipMap(_, _) => {
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
            SetFromSprite(sprite_id, bounds)                => { /* todo!() */ },
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

        // Build the texture structure
        let pixels  = vec![0u8; width * height * 4];
        let texture = RgbaTexture::from_pixels(width, height, pixels);
        let texture = Texture::Rgba(Arc::new(texture));

        // Store it, replacing any existing texture with this ID
        self.textures.insert((self.current_namespace, texture_id), Arc::new(texture));
    }

    ///
    /// Sets the bytes for a region of the texture
    ///
    #[inline]
    pub (crate) fn texture_set_bytes(&mut self, texture_id: canvas::TextureId, canvas::TexturePosition(x, y): canvas::TexturePosition, canvas::TextureSize(width, height): canvas::TextureSize, bytes: Arc<Vec<u8>>) {
        if let Some(texture) = self.textures.get_mut(&(self.current_namespace, texture_id)) {
            // The texture exists: prepare to write to it
            let texture     = Arc::make_mut(texture);
            let x           = x as usize;
            let y           = y as usize;
            let width       = width as usize;
            let height      = height as usize;

            // How the bytes are written depend on the format of the texture
            match texture {
                Texture::Rgba(rgba) => {
                    let rgba = Arc::make_mut(rgba);
                    rgba.set_bytes(x, y, width, height, &*bytes);
                }

                Texture::MipMap(rgba, _) => {
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
        let textures        = &self.textures;
        let current_state   = &mut self.current_state;
        let data_cache      = &mut self.program_data_cache;

        // Transform the coordiantes to screen coordinates
        let (x1, y1) = current_state.transform.transform_point(x1, y1);
        let (x2, y2) = current_state.transform.transform_point(x2, y2);

        if let Some(texture) = textures.get(&(self.current_namespace, texture_id)) {
            // Texture exists
            match &**texture {
                Texture::Rgba(rgba_texture) => {
                    // We want to make a transformation that maps x1, y1 to 0,0 and x2, y2 to w, h
                    let w = rgba_texture.width() as f32;
                    let h = rgba_texture.height() as f32;

                    let transform = canvas::Transform2D::translate(-x1, -y1);
                    let transform = canvas::Transform2D::scale(1.0/(x2-x1), 1.0/(y2-y1)) * transform;
                    let transform = canvas::Transform2D::scale(w, h) * transform;

                    debug_assert!((transform.transform_point(x1, y1).0 - 0.0).abs() < 0.01, "{:?} {:?}", transform.transform_point(x1, y1), (0.0, 0.0));
                    debug_assert!((transform.transform_point(x2, y2).1 - h).abs() < 0.01, "{:?} {:?}", transform.transform_point(x2, y2), (w, h));

                    // Set as the brush state
                    DrawingState::release_program(&mut current_state.fill_program, data_cache);
                    current_state.next_fill_brush = Brush::TransparentTexture(Arc::clone(rgba_texture), transform);
                },

                Texture::MipMap(_, mipmap) => {
                    // We want to make a transformation that maps x1, y1 to 0,0 and x2, y2 to w, h
                    let w = mipmap.mip_level(0).width() as f32;
                    let h = mipmap.mip_level(0).height() as f32;

                    let transform = canvas::Transform2D::translate(-x1, -y1);
                    let transform = canvas::Transform2D::scale(1.0/(x2-x1), 1.0/(y2-y1)) * transform;
                    let transform = canvas::Transform2D::scale(w, h) * transform;

                    debug_assert!((transform.transform_point(x1, y1).0 - 0.0).abs() < 0.01, "{:?} {:?}", transform.transform_point(x1, y1), (0.0, 0.0));
                    debug_assert!((transform.transform_point(x2, y2).1 - h).abs() < 0.01, "{:?} {:?}", transform.transform_point(x2, y2), (w, h));

                    // Set as the brush state
                    DrawingState::release_program(&mut current_state.fill_program, data_cache);
                    current_state.next_fill_brush = Brush::TransparentMipMapTexture(Arc::clone(mipmap), transform);
                }
            }
        }
    }
}
