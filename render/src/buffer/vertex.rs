///
/// 2D vertex representation
///
#[derive(Clone, Copy, PartialEq, Debug)]
#[repr(C, packed)]
pub struct Vertex2D {
    pub pos:        [f32; 2],
    pub tex_coord:  [f32; 2],
    pub color:      [u8; 4]
}

impl Vertex2D {
    ///
    /// Creates a 2D vertex with the position set and the other values zeroed out
    ///
    pub fn with_pos(x: f32, y: f32) -> Vertex2D {
        Vertex2D {
            pos:        [x, y],
            tex_coord:  [0.0, 0.0],
            color:      [0, 0, 0, 0]
        }
    }
}