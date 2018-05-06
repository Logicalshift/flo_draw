///
/// Represents a 2D affine transformation matrix
///
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Transform2D(pub (f32, f32, f32), pub (f32, f32, f32), pub (f32, f32, f32));

impl Transform2D {
    pub fn identity() -> Transform2D {
        Transform2D((1.0, 0.0, 0.0), (0.0, 1.0, 0.0), (0.0, 0.0, 1.0))
    }

    pub fn translate(x: f32, y: f32) -> Transform2D {
        Transform2D((1.0, 0.0, x), (0.0, 1.0, y), (0.0, 0.0, 1.0))
    }
}
