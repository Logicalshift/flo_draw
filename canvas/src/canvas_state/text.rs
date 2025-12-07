use super::brush::*;
use super::texture::*;
use super::gradient::*;

use crate::draw::*;
use crate::font::*;
use crate::font_face::*;

use std::sync::*;

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
