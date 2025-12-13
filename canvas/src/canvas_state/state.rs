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

impl Default for CanvasState {
    ///
    /// Creates a canvas in the default state
    ///
    fn default() -> Self {
        CanvasState { 
            background:         Color::Rgba(1.0, 1.0, 1.0, 1.0), 
            current_brush:      CanvasShared::new(CanvasBrush::default()), 
            state_stack:        vec![], 
            layers_and_sprites: HashMap::new(), 
            clear_count:        HashMap::new(), 
            textures:           HashMap::new(), 
            gradients:          HashMap::new(), 
            fonts:              HashMap::new(), 
            current_path:       CanvasPath::default(), 
            current_text:       vec![],
        }
    }
}
