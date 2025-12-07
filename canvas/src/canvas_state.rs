use crate::draw::*;
use crate::path::*;
use crate::color::*;
use crate::gradient::*;
use crate::transform2d::*;

use std::collections::*;
use std::sync::*;

///
/// Contains the current state of a canvas (so its drawing instructions can be replicated or relayed)
///
#[derive(Clone)]
pub struct CanvasState {
    drawing_target:     DrawingTarget,
    current_brush:      SharedCanvasBrush,
    state_stack:        Vec<Arc<CanvasBrush>>,

    layers_and_sprites: HashMap<DrawingTarget, Vec<CanvasEntity>>,

    /*
    textures:
    gradients:
    fonts:
     */
}

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
/// Where the next item to be drawn should go
///
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum DrawingTarget {
    Layer(LayerId),
    Sprite(SpriteId),
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
}

///
/// Definition of a path
///
#[derive(Clone, Debug, PartialEq)]
pub struct CanvasPath {
    operations: Vec<PathOp>,
}

///
/// An entity rendered on a canvas
///
#[derive(Clone, Debug)]
pub struct CanvasEntity {
    brush:  Arc<CanvasBrush>,
    path:   CanvasPath
}

///
/// Canvas brush that can be modified or shared
///
#[derive(Clone, Debug)]
pub enum SharedCanvasBrush {
    Modified(CanvasBrush),
    Shared(Arc<CanvasBrush>)
}
