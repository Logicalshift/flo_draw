use super::font_face::*;

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
/// Operations that can be performed on a font
///
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum FontOp { 
    /// Loads a font from a font data file
    UseFontDefinition(Arc<CanvasFontFace>),

    /// Sets the font size to use for this font ID (in canvas units)
    FontSize(f32)
}
