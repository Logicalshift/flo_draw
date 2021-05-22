use super::draw::*;

///
/// Operations that define paths
///
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum PathOp {
    /// Begins a new path
    NewPath,

    /// Move to a new point
    Move(f32, f32),

    /// Line to point
    Line(f32, f32),

    /// Bezier curve to point
    BezierCurve(((f32, f32), (f32, f32)), (f32, f32)),

    /// Closes the current subpath
    ClosePath,
}

impl Into<Draw> for PathOp {
    #[inline]
    fn into(self) -> Draw {
        Draw::Path(self)
    }
}
