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
}