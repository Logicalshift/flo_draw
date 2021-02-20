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
/// Data for a font definition
///
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum FontData {
    /// Binary data representing a TTF format font file
    Ttf(Arc<Vec<u8>>),

    /// Binary data representing an OTF format font file
    Otf(Arc<Vec<u8>>)
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

///
/// Operations that can be performed on a font
///
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum FontOp { 
    /// Associates a system font with this font ID
    UseSystemFont(String, FontProperties),

    /// Loads a font from a font data file
    UseFontDefinition(FontData),

    /// Sets the font size to use for this font ID (in canvas units)
    FontSize(f32)
}
