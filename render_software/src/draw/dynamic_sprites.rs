use super::canvas_drawing::*;
use super::drawing_state::*;

use crate::filters::PixelFilter;
use crate::pixel::*;

use flo_canvas as canvas;

use std::sync::*;

///
/// A dynamic sprite is a sprite that is rendered to a texture on demand, where the texture's size is determined by the current render resolution
/// rather than being measured in pixels
///
#[derive(Clone)]
pub (crate) struct DynamicSprite {
    sprite_id: canvas::SpriteId,

    /// The region in sprite coordinates to render
    sprite_bounds: canvas::SpriteBounds,

    /// The size of the final texture, in canvas units
    canvas_bounds: canvas::CanvasSize,

    /// The filters that are applied to this dynamic sprite
    filters: Vec<Arc<dyn Send + Sync + Fn(Arc<U16LinearTexture>) -> U16LinearTexture>>,

    /// The most recent render of this sprite (or None if it has never been rendered)
    last_render: Option<Arc<U16LinearTexture>>,

    /// The last pixel size used in canvas units, used to determine if this texture needs to be re-rendered
    last_pixel_size: f64,

    /// The layer edit count used the last time this sprite was re-rendered
    last_render_layer_count: usize,
}

impl DynamicSprite {
    ///
    /// Retrieves the texture (possibly rendering it if needed)
    ///
    pub fn get_u16_texture<TPixel, const N: usize>(&mut self, drawing: &CanvasDrawing<TPixel, N>) -> Arc<U16LinearTexture>
    where
        TPixel: Pixel<N>
    {
        todo!()
    }

    ///
    /// Adds a filter to the dynamic sprite
    ///
    pub fn apply_filter<TPixel, const N: usize>(&mut self, filter: impl 'static + Send + Sync + PixelFilter<Pixel=TPixel>)
    where
        TPixel: Pixel<N>
    {
        // todo
    }
}

impl<TPixel, const N: usize> CanvasDrawing<TPixel, N>
where
    TPixel: 'static + Send + Sync + Pixel<N>,
{
    ///
    /// Creates a dynamic texture.
    ///
    /// These are rendered at the resolution of the output, and are re-rendered whenever the resolution or the sprite changes.
    /// Filters are re-applied when re-rendering. The textures are rendered just before they are used and are not re-rendered
    /// unless the parameters change.
    ///
    pub fn texture_create_dynamic_sprite(&mut self, texture_id: canvas::TextureId, sprite_id: canvas::SpriteId, bounds: canvas::SpriteBounds, size: canvas::CanvasSize) {
        // Create a structure to represent the dynamic sprite, which is not currently rendered
        let new_sprite = DynamicSprite {
            sprite_id:                  sprite_id,
            sprite_bounds:              bounds,
            canvas_bounds:              size,
            filters:                    vec![],
            last_render:                None,
            last_pixel_size:            0.0,
            last_render_layer_count:    0
        };

        // todo
    }
}
