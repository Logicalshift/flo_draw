use super::path::*;
use super::text::*;
use super::brush::*;
use super::share::*;
use super::entity::*;
use super::texture::*;
use super::gradient::*;
use super::drawing_target::*;

use crate::draw::*;
use crate::color::*;
use crate::gradient::*;
use crate::font_face::*;

use std::collections::*;
use std::sync::*;

///
/// Contains the current state of a canvas (so its drawing instructions can be replicated or relayed)
///
#[derive(Clone)]
pub struct CanvasState {
    background:         Color,

    drawing_target:     DrawingTarget,
    current_brush:      CanvasShared<CanvasBrush>,
    state_stack:        Vec<Arc<CanvasBrush>>,

    layers_and_sprites: HashMap<DrawingTarget, Vec<CanvasEntity>>,
    clear_count:        HashMap<DrawingTarget, usize>,
    textures:           HashMap<TextureId, CanvasShared<CanvasTexture>>,
    gradients:          HashMap<GradientId, CanvasShared<CanvasGradient>>,
    fonts:              HashMap<FontId, Arc<CanvasFontFace>>,

    current_path:       CanvasPath,
    current_text:       Vec<CanvasTextLayout>,
}
