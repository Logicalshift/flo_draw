use super::color::*;

///
/// Identifies a gradient
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GradientId(pub u64);

///
/// Operations that can be applied to a gradient
///
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GradientOp {
    /// Clears the gradient and starts a new one with the given initial colour
    New(Color),

    /// Adds a new gradient stop of the specified colour
    AddStop(f32, Color)
}
