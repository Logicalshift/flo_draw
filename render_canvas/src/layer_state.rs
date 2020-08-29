use super::stroke_settings::*;

use flo_canvas as canvas;
use flo_render as render;

///
/// The current state of a layer
///
#[derive(Clone)]
pub struct LayerState {
    /// The current fill colour
    pub fill_color: render::Rgba8,

    /// The blend mode set for this layer
    pub blend_mode: canvas::BlendMode,

    /// The settings for the next brush stroke
    pub stroke_settings: StrokeSettings,

    /// Where the canvas's rendering should be rolled back to on the next 'restore' operation
    pub restore_point: Option<usize>,

    /// The current transformation matrix for this layer
    pub current_matrix: canvas::Transform2D,

    /// The current transform to apply when rendering sprites
    pub sprite_matrix: canvas::Transform2D
}

impl LayerState {
    ///
    /// Applies a sprite transformation to this state
    ///
    pub fn apply_sprite_transform(&mut self, transform: canvas::SpriteTransform) {
        match transform {
            canvas::SpriteTransform::Identity   => { self.sprite_matrix = canvas::Transform2D::identity(); },
            other                               => { self.sprite_matrix = self.sprite_matrix * canvas::Transform2D::from(other); }
        }
    }
}