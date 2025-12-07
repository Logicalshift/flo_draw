use crate::color::*;

///
/// Canvas gradient definition
///
#[derive(Clone, Debug)]
pub struct CanvasGradient {
    stops: Vec<(f32, Color)>,
}
