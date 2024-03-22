use super::canvas_drawing::*;
use super::drawing_state::*;

use crate::pixel::*;

use flo_canvas as canvas;

impl<TPixel, const N: usize> CanvasDrawing<TPixel, N>
where
    TPixel: 'static + Send + Sync + Pixel<N>,
{
    ///
    /// Creates a dynamic texture.
    ///
    /// These are rendered at the resolution of the output, and are re-rendered whenever the resolution or the sprite changes.
    /// Filters are re-applied when re-rendering.
    ///
    pub fn texture_create_dynamic_sprite(&mut self, texture_id: canvas::TextureId, sprite_id: canvas::SpriteId, bounds: canvas::SpriteBounds, size: canvas::CanvasSize) {
        // todo
    }
}
