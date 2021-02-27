use super::font_face::*;

use flo_curves::geo::*;

use std::sync::*;

///
/// The possible styles of a font
///
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum FontStyle {
    Normal,
    Italic,
    Oblique
}

///
/// The properties to use when selecting a font face
///
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct FontProperties {
    pub style: FontStyle,
    pub weight: u32
}

impl Default for FontStyle {
    fn default() -> FontStyle { FontStyle::Normal }
}

impl Default for FontProperties {
    fn default() -> FontProperties { FontProperties { style: FontStyle::default(), weight: 400 } }
}

impl FontProperties {
    ///
    /// Returns an updated font properties object with a new weight
    ///
    pub fn with_weight(mut self, new_weight: u32) -> FontProperties {
        self.weight = new_weight;
        self
    }

    ///
    /// Returns an updated font properties object with a new style
    ///
    pub fn with_style(mut self, new_style: FontStyle) -> FontProperties {
        self.style = new_style;
        self
    }
}

///
/// Determines how text is drawn relative to its alignment's origin point
///
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum TextAlignment {
    Left,
    Right,
    Center
}

///
/// Operations that can be performed on a font
///
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum FontOp { 
    /// Loads a font from a font data file
    UseFontDefinition(Arc<CanvasFontFace>),

    /// Sets the font size to use for this font ID (in canvas units)
    FontSize(f32),

    /// Lays out some text in the active layout, to be rendered in the current fill style
    LayoutText(String),

    /// Draws a series of glyphs using the current fill style
    DrawGlyphs(Vec<GlyphPosition>)
}

///
/// The layout metrics for a piece of text
///
#[derive(Clone, PartialEq)]
pub struct TextLayoutMetrics {
    /// The bounding box of the text that was laid out - using the height of the font and the offsets of the glyphs
    pub inner_bounds: Bounds<Coord2>,

    /// The overall bounding box of the text that was laid out
    pub outer_bounds: Bounds<Coord2>
}

///
/// ID for a glyph within a font
///
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct GlyphId(pub u32);

///
/// Describes how a glyph is positioned on the canvas
///
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct GlyphPosition {
    /// The ID of the glyph to render
    pub id: GlyphId,

    /// Position of the glyph's baseline
    pub location: (f32, f32),

    /// The number of canvas units that map to one em in font units
    pub em_size: f32
}
