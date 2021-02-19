use super::stroke_settings::*;

use flo_canvas as canvas;
use flo_render as render;

use lyon::tessellation::{FillRule};

///
/// The current state of a layer
///
#[derive(Clone)]
pub struct LayerState {
    /// True if this layer contains a sprite
    pub is_sprite: bool,

    /// The current fill colour
    pub fill_color: render::Rgba8,

    /// The fill rule to use
    pub winding_rule: FillRule,

    /// The blend mode set for this layer
    pub blend_mode: canvas::BlendMode,

    /// The settings for the next brush stroke
    pub stroke_settings: StrokeSettings,

    /// Where the canvas's rendering should be rolled back to on the next 'restore' operation
    pub restore_point: Option<usize>,

    /// The current transformation matrix for this layer
    pub current_matrix: canvas::Transform2D,

    /// The scale factor applied by the current matrix (used to determine the precision of the tessellator)
    pub scale_factor: f32,

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

    ///
    /// Returns the scale factor to use for fills and strokes given a particular viewport height
    ///
    pub fn tolerance_scale_factor(&mut self, viewport_height: f32) -> f64 {
        // Assume the viewport is at least a certain size (so if the rendering is initially to a very small viewport during initialisation we won't produce a wildly inaccurate rendering)
        let _viewport_height = if viewport_height < 1000.0 {
            1000.0
        } else {
            viewport_height
        };

        1.0
    }
}