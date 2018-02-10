///
/// Represents a 2D affine transformation matrix
///
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Transform2D(pub (f32, f32, f32), pub (f32, f32, f32), pub (f32, f32, f32));
