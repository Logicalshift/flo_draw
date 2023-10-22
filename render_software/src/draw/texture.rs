use super::canvas_drawing::*;

use crate::pixel::*;

use flo_canvas as canvas;

use std::sync::*;

///
/// The data stored as part of a texture
///
#[derive(Clone)]
pub enum Texture {
    /// A texture in Rgba format
    Rgba(RgbaTexture),
}

impl RgbaTexture {
    ///
    /// Sets the bytes for a region of this image
    ///
    #[inline]
    pub fn set_bytes(&mut self, x: usize, y: usize, width: usize, height: usize, bytes: &Vec<u8>) {
        // Number of bytes to read/write at a time
        let mut read_bytes = width * 4;

        // Number of bytes to skip after every read_bytes
        let mut read_skip_bytes = 0;

        // Clip to the size of the image
        let x       = x.min(self.width);
        let y       = y.min(self.height);
        let width   = if x + width > self.width {
            let clip        = (x + width) - self.width;
            read_bytes      = (width - clip) * 4;
            read_skip_bytes = clip * 4;
            width - clip
        } else {
            width 
        };
        let height  = height.min(self.height - y);

        // After writing read_bytes, skip this many bytes to write
        let write_skip_bytes = (self.width - width) * 4;

        // Pointers for reading/writing
        let mut write_idx   = (x*4) + (y*self.width*4);
        let mut read_idx    = 0;

        for _ in 0..height {
            // Write a row
            for _ in 0..read_bytes {
                self.pixels[write_idx] = bytes[read_idx];
                write_idx   += 1;
                read_idx    += 1;
            }

            // Skip to the next row
            read_idx    += read_skip_bytes;
            write_idx   += write_skip_bytes;
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
    pub fn texture(&mut self, texture_id: canvas::TextureId, texture_op: canvas::TextureOp) {
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
    pub fn texture_free(&mut self, texture_id: canvas::TextureId) {
        self.textures.remove(&(self.current_namespace, texture_id));
    }

    ///
    /// Creates a blank RGBA texture of a particular size
    ///
    #[inline]
    pub fn texture_create_rgba(&mut self, texture_id: canvas::TextureId, canvas::TextureSize(width, height): canvas::TextureSize) {
        let width   = width as usize;
        let height  = height as usize;

        // Build the texture structure
        let pixels  = vec![0u8; width * height * 4];
        let texture = RgbaTexture { width, height, pixels };
        let texture = Texture::Rgba(texture);

        // Store it, replacing any existing texture with this ID
        self.textures.insert((self.current_namespace, texture_id), Arc::new(texture));
    }

    #[inline]
    pub fn texture_set_bytes(&mut self, texture_id: canvas::TextureId, canvas::TexturePosition(x, y): canvas::TexturePosition, canvas::TextureSize(width, height): canvas::TextureSize, bytes: Arc<Vec<u8>>) {
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
                    rgba.set_bytes(x, y, width, height, &*bytes);
                }
            }
        }
    }
}
