use crate::draw::*;
use crate::font::*;
use crate::path::*;
use crate::color::*;
use crate::texture::*;
use crate::gradient::*;
use crate::font_face::*;
use crate::transform2d::*;

use std::collections::*;
use std::sync::*;

///
/// Contains the current state of a canvas (so its drawing instructions can be replicated or relayed)
///
#[derive(Clone)]
pub struct CanvasState {
    drawing_target:     DrawingTarget,
    current_brush:      CanvasShared<CanvasBrush>,
    state_stack:        Vec<Arc<CanvasBrush>>,

    layers_and_sprites: HashMap<DrawingTarget, Vec<CanvasEntity>>,
    textures:           HashMap<TextureId, CanvasShared<CanvasTexture>>,
    gradients:          HashMap<GradientId, CanvasShared<CanvasGradient>>,
    fonts:              HashMap<FontId, Arc<CanvasFontFace>>,

    current_path:       CanvasPath,
    current_text:       Vec<CanvasTextLayout>,
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

    font_size:          f32,
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
pub enum CanvasEntity {
    FillPath {
        brush:      Arc<CanvasBrush>,
        texture:    Option<Arc<CanvasTexture>>,
        gradient:   Option<Arc<CanvasGradient>>,
        path:       CanvasPath,
    },

    StrokePath {
        brush:      Arc<CanvasBrush>,
        texture:    Option<Arc<CanvasTexture>>,
        gradient:   Option<Arc<CanvasGradient>>,
        path:       CanvasPath,
    },

    TextLayout {
        text:       Vec<CanvasTextLayout>,
    }
}

///
/// Canvas brush that can be modified or shared
///
#[derive(Clone, Debug)]
pub enum CanvasShared<T> {
    Modified(T),
    Shared(Arc<T>)
}

///
/// Canvas texture definition
///
/// Texture data is optional here: we won't be able to reconstruct textures if this is not supplied but
/// also won't duplicate them in the renderer (or spend time copying the bytes into multiple buffers)
///
#[derive(Clone, Debug)]
pub struct CanvasTexture {
    format:             TextureFormat,
    size:               (f32, f32),
    bytes:              CanvasTextureData,
    fill_transparency:  f32,
}

///
/// Data storage for a texture
///
#[derive(Clone, Debug)]
pub enum CanvasTextureData {
    None,
    Modified(Vec<u8>),
    Shared(Arc<Vec<u8>>),
}

///
/// Canvas gradient definition
///
#[derive(Clone, Debug)]
pub struct CanvasGradient {
    stops: Vec<(f32, Color)>,
}

///
/// Laid out text
///
#[derive(Clone, Debug)]
pub struct CanvasTextLayout {
    pos:    (f32, f32),
    text:   Vec<CanvasTextChunk>,
}

///
/// Block of text that is laid out
///
#[derive(Clone, Debug)]
pub struct CanvasTextChunk {
    text:       Vec<String>,

    font_defn:  Arc<CanvasFontFace>,
    font_id:    FontId,
    brush:      Arc<CanvasBrush>,
    texture:    Option<Arc<CanvasTexture>>,
    gradient:   Option<Arc<CanvasGradient>>,
    alignment:  TextAlignment,
}
