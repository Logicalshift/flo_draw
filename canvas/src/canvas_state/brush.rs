use super::path::*;
use super::drawing_target::*;

use crate::draw::*;
use crate::color::*;
use crate::gradient::*;
use crate::transform2d::*;

///
/// Describes the currently selected fill
///
#[derive(Clone, Debug)]
pub enum FillState {
    Color(Color),
    Texture(TextureId, (f32, f32), (f32, f32)),
    Gradient(GradientId, (f32, f32), (f32, f32)),
}

///
/// Describes the selected line width
///
#[derive(Clone, Debug)]
pub enum LineWidth {
    Width(f32),
    WidthPixels(f32),
}

///
/// The 'brush' for a canvas describes the drawing state (what will be done on commands like `Fill` or `Stroke`)
///
#[derive(Clone, Debug)]
pub struct CanvasBrush {
    pub(super) target:             DrawingTarget,

    pub(super) fill:               FillState, 
    pub(super) fill_transform:     Transform2D,
    pub(super) winding_rule:       WindingRule,

    pub(super) stroke_color:       Color,
    pub(super) line_join:          LineJoin,
    pub(super) line_cap:           LineCap,
    pub(super) line_width:         LineWidth,
    pub(super) dash_pattern:       Vec<f32>,
    pub(super) dash_offset:        f32,

    pub(super) blend_mode:         BlendMode,

    pub(super) canvas_height:      f32,
    pub(super) center_region:      Option<((f32, f32), (f32, f32))>,
    pub(super) multiply_transform: Transform2D,

    pub(super) clip_path:          Vec<CanvasPath>,

    pub(super) sprite_transform:   SpriteTransform,

    pub(super) font_size:          f32,
}

impl Default for CanvasBrush {
    ///
    /// Creates the default 
    ///
    fn default() -> Self {
        CanvasBrush {
            target:             DrawingTarget::Layer(LayerId(0)),
            fill:               FillState::Color(Color::Rgba(0.0, 0.0, 0.0, 1.0)),
            fill_transform:     Transform2D::identity(),
            winding_rule:       WindingRule::NonZero,
            stroke_color:       Color::Rgba(0.0, 0.0, 0.0, 1.0),
            line_join:          LineJoin::Round,
            line_cap:           LineCap::Butt,
            line_width:         LineWidth::Width(1.0),
            dash_pattern:       vec![],
            dash_offset:        0.0,
            blend_mode:         BlendMode::SourceOver,
            canvas_height:      2.0,
            center_region:      None,
            multiply_transform: Transform2D::identity(),
            clip_path:          vec![],
            sprite_transform:   SpriteTransform::Identity,
            font_size:          12.0,
        }
    }
}

impl CanvasBrush {
    ///
    /// Returns the drawing target of this brush
    ///
    #[inline]
    pub fn drawing_target(&self) -> DrawingTarget {
        self.target.clone()
    }

    ///
    /// Returns the ID of the currently set fill texture
    ///
    pub fn fill_texture(&self) -> Option<TextureId> {
        match &self.fill {
            FillState::Texture(id, _, _)    => Some(*id),
            _                               => None,
        }
    }

    ///
    /// Returns the ID of the currently set fill texture
    ///
    pub fn fill_gradient(&self) -> Option<GradientId> {
        match &self.fill {
            FillState::Gradient(id, _, _)   => Some(*id),
            _                               => None,
        }
    }
}
