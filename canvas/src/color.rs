use hsluv::*;

///
/// Representation of a colour 
///
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Color {
    Rgba(f32, f32, f32, f32),
    Hsluv(f32, f32, f32, f32)
}

impl Color {
    ///
    /// Returns this colour as RGBA components
    /// 
    pub fn to_rgba_components(&self) -> (f32, f32, f32, f32) {
        match self {
            &Color::Rgba(r, g, b, a) => (r, g, b, a),

            &Color::Hsluv(h, s, l, a) => {
                let (r, g, b) = hsluv_to_rgb((h as f64, s as f64, l as f64));
                (r as f32, g as f32, b as f32, a)
            }
        }
    }

    ///
    /// Returns a new colour that's the same as this one except with a different alpha value
    /// 
    pub fn with_alpha(&self, new_alpha: f32) -> Color {
        match self {
            &Color::Rgba(r, g, b, _)    => Color::Rgba(r, g, b, new_alpha),
            &Color::Hsluv(h, s, l, _)   => Color::Hsluv(h, s, l, new_alpha)
        }
    }
}
