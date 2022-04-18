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

    ///
    /// Updates this vertex with a particular colour
    ///
    pub fn with_color(self, r: f32, g: f32, b: f32, a: f32) -> Vertex2D {
        Vertex2D {
            pos:        self.pos,
            tex_coord:  self.tex_coord,
            color:      [(r*255.0) as _, (g*255.0) as _, (b*255.0) as _, (a*255.0) as _]
        }
    }

    ///
    /// Updates this vertex with a texture coordinate
    ///
    pub fn with_texture_coordinates(self, x: f32, y: f32) -> Vertex2D {
        Vertex2D {
            pos:        self.pos,
            tex_coord:  [x, y],
            color:      self.color
        }
    }
}