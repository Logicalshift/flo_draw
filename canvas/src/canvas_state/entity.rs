use super::path::*;
use super::text::*;
use super::brush::*;
use super::texture::*;
use super::gradient::*;

use std::sync::*;

///
/// An entity rendered on a canvas, such as a layer or a sprite
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
