///
/// Representation of a colour 
///
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Color {
    Rgba(f32, f32, f32, f32)
}

impl Color {
    ///
    /// Returns this colour as RGBA components
    /// 
    pub fn to_rgba(&self) -> (f32, f32, f32, f32) {
        match self {
            &Color::Rgba(r, g, b, a) => (r, g, b, a)
        }
    }

    ///
    /// Returns a new colour that's the same as this one except with a different alpha value
    /// 
    pub fn with_alpha(&self, new_alpha: f32) -> Color {
        match self {
            &Color::Rgba(r, g, b, _) => Color::Rgba(r, g, b, new_alpha)
        }
    }
}
