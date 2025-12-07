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
    target:             DrawingTarget,

    fill:               FillState, 
    fill_transform:     Transform2D,
    winding_rule:       WindingRule,

    stroke_color:       Color,
    line_join:          LineJoin,
    line_cap:           LineCap,
    line_width:         LineWidth,
    dash_pattern:       Vec<f32>,
    dash_offset:        f32,

    blend_mode:         BlendMode,

    canvas_height:      f32,
    center_region:      Option<((f32, f32), (f32, f32))>,
    multiply_transform: Transform2D,

    clip_path:          Option<CanvasPath>,

    sprite_transform:   SpriteTransform,

    font_size:          f32,
}
