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
