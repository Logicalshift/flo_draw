use super::fill_state::*;
use super::stroke_settings::*;

use flo_canvas as canvas;

use lyon::tessellation::{FillRule};

///
/// The current state of a layer
///
#[derive(Clone)]
pub struct LayerState {
    /// True if this layer contains a sprite
    pub is_sprite: bool,

    /// The current fill colour
    pub fill_color: FillState,

    /// The alpha value to use with textures
    pub texture_alpha: f32,

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
        let viewport_height = if viewport_height < 1000.0 {
            1000.0
        } else {
            viewport_height as f64
        };

        let scale_factor = self.scale_factor as f64;
        let scale_factor = if scale_factor.abs() < 0.000001 { 0.000001 } else { scale_factor };

        // The window height is 2.0 - so 2.0/scale_factor = the height of the viewport with the current transformation. We use 4.0 instead of 2.0 to reduce the precision a bit for rendering.
        (4.0/scale_factor) / viewport_height
    }
}