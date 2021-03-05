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
/// Describes where to position a line relative to some text
///
#[derive(Copy, Clone, PartialEq)]
pub struct FontLinePosition {
    pub offset:     f32,
    pub thickness:  f32
}

///
/// Metrics for a font
///
#[derive(Copy, Clone, PartialEq)]
pub struct FontMetrics {
    /// Size of an em relative to these metrics
    pub em_size:            f32,

    /// The ascender size for the font
    pub ascender:           f32,

    /// The descender size for the font
    pub descender:          f32,

    /// The height for the font
    pub height:             f32,

    /// The line gap for the font
    pub line_gap:           f32,

    /// The capital height for the font, if specified
    pub capital_height:     Option<f32>,

    /// Offset from the baseline and suggested thickness for an underline (can be None if the font does not specify)
    pub underline_position: Option<FontLinePosition>,

    /// Offset from the baseline and suggested thickness for a strikeout effect
    pub strikeout_position: Option<FontLinePosition>,
}

impl FontMetrics {
    ///
    /// Returns the metrics adjusted to a new em size
    ///
    pub fn with_size(self, em_size: f32) -> FontMetrics {
        let scale_factor = em_size / self.em_size;

        FontMetrics {
            em_size:            self.em_size * scale_factor,
            ascender:           self.ascender * scale_factor,
            descender:          self.descender * scale_factor,
            height:             self.height * scale_factor,
            line_gap:           self.line_gap * scale_factor,
            capital_height:     self.capital_height.map(|height| height*scale_factor),
            underline_position: self.underline_position.map(|mut pos| { pos.offset *= scale_factor; pos.thickness *= scale_factor; pos }),
            strikeout_position: self.strikeout_position.map(|mut pos| { pos.offset *= scale_factor; pos.thickness *= scale_factor; pos }),
        }
    }
}

///
/// The layout metrics for a piece of text
///
#[derive(Copy, Clone, PartialEq)]
pub struct TextLayoutMetrics {
    /// The bounding box of the text that was laid out - using the height of the font and the offsets of the glyphs
    pub inner_bounds: (Coord2, Coord2),

    /// The point where the next glyph will be positioned
    pub pos: Coord2
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
