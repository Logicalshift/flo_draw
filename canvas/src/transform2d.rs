use super::draw::{SpriteTransform};
use std::f32;
use std::ops::{Mul};

///
/// Represents a 2D affine transformation matrix
///
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Transform2D(pub [[f32; 3]; 3]);

impl Transform2D {
    ///
    /// Creates the identity transform
    ///
    pub fn identity() -> Transform2D {
        Transform2D([
            [1.0, 0.0, 0.0], 
            [0.0, 1.0, 0.0], 
            [0.0, 0.0, 1.0]])
    }

    ///
    /// Creates a translation transformation
    ///
    pub fn translate(x: f32, y: f32) -> Transform2D {
        Transform2D([
            [1.0, 0.0, x    ], 
            [0.0, 1.0, y    ], 
            [0.0, 0.0, 1.0  ]
        ])
    }

    ///
    /// Creates a scaling transformation
    ///
    pub fn scale(scale_x: f32, scale_y: f32) -> Transform2D {
        Transform2D([
            [scale_x,   0.0,        0.0], 
            [0.0,       scale_y,    0.0], 
            [0.0,       0.0,        1.0]])
    }

    ///
    /// Creates a rotation transformation
    ///
    pub fn rotate(radians: f32) -> Transform2D {
        let cos = f32::cos(radians);
        let sin = f32::sin(radians);

        Transform2D([
            [cos,   -sin,   0.0],
            [sin,   cos,    0.0],
            [0.0,   0.0,    1.0]
        ])
    }

    ///
    /// Computes the determinant of a 2x2 matrix
    ///
    fn det2(matrix: &[[f32; 2]; 2]) -> f32 {
        matrix[0][0]*matrix[1][1] + matrix[0][1]*matrix[1][0]
    }

    ///
    /// Computes the minor of a 3x3 matrix
    ///
    fn minor3(matrix: &[[f32; 3]; 3], row: usize, col: usize) -> f32 {
        let (x1, x2)    = match row { 0 => (1, 2), 1 => (0, 2), 2 => (0, 1), _ => (0, 1) };
        let (y1, y2)    = match col { 0 => (1, 2), 1 => (0, 2), 2 => (0, 1), _ => (0, 1) };

        let matrix      = [
            [matrix[y1][x1], matrix[y1][x2]], 
            [matrix[y2][x1], matrix[y2][x2]]
        ];

        Self::det2(&matrix)
    }

    ///
    /// Computes the cofactor of an element in a 3x3 matrix
    ///
    fn cofactor3(matrix: &[[f32; 3]; 3], row: usize, col: usize) -> f32 {
        let minor   = Self::minor3(matrix, row, col);
        let sign    = (col&1) ^ (row&1);

        if sign != 0 {
            -minor 
        } else {
            minor
        }
    }

    ///
    /// Inverts a matrix transform
    ///
    fn invert_matrix(matrix: &[[f32; 3]; 3]) -> Option<[[f32; 3]; 3]> {
        let cofactors   = [
            [Self::cofactor3(&matrix, 0, 0), Self::cofactor3(&matrix, 1, 0), Self::cofactor3(&matrix, 2, 0)],
            [Self::cofactor3(&matrix, 0, 1), Self::cofactor3(&matrix, 1, 1), Self::cofactor3(&matrix, 2, 1)],
            [Self::cofactor3(&matrix, 0, 2), Self::cofactor3(&matrix, 1, 2), Self::cofactor3(&matrix, 2, 2)],
        ];

        let det         = matrix[0][0]*cofactors[0][0] + matrix[0][1]*cofactors[0][1] + matrix[0][2]*cofactors[0][2];

        if det != 0.0 {
            let inv_det = 1.0/det;

            Some([
                [inv_det * cofactors[0][0], inv_det * cofactors[1][0], inv_det * cofactors[2][0]],
                [inv_det * cofactors[0][1], inv_det * cofactors[1][1], inv_det * cofactors[2][1]],
                [inv_det * cofactors[0][2], inv_det * cofactors[1][2], inv_det * cofactors[2][2]]
            ])
        } else {
            None
        }
    }

    ///
    /// Returns an inverted Transform2D
    ///
    pub fn invert(&self) -> Option<Transform2D> {
        let Transform2D(matrix) = self;

        Self::invert_matrix(matrix)
            .map(|inverted| Transform2D(inverted))
    }
}

impl Mul<Transform2D> for Transform2D {
    type Output=Transform2D;

    fn mul(self, other: Transform2D) -> Transform2D {
        let Transform2D(a) = self;
        let Transform2D(b) = other;

        Transform2D([
            [a[0][0]*b[0][0] + a[0][1]*b[1][0] + a[0][2]*b[2][0],   a[0][0]*b[0][1] + a[0][1]*b[1][1] + a[0][2]*b[2][1],    a[0][0]*b[0][2] + a[0][1]*b[1][2] + a[0][2]*b[2][2]],
            [a[1][0]*b[0][0] + a[1][1]*b[1][0] + a[1][2]*b[2][0],   a[1][0]*b[0][1] + a[1][1]*b[1][1] + a[1][2]*b[2][1],    a[1][0]*b[0][2] + a[1][1]*b[1][2] + a[1][2]*b[2][2]],
            [a[2][0]*b[0][0] + a[2][1]*b[1][0] + a[2][2]*b[2][0],   a[2][0]*b[0][1] + a[2][1]*b[1][1] + a[2][2]*b[2][1],    a[2][0]*b[0][2] + a[2][1]*b[1][2] + a[2][2]*b[2][2]],
        ])
    }
}

impl Mul<&Transform2D> for &Transform2D {
    type Output=Transform2D;

    fn mul(self, other: &Transform2D) -> Transform2D {
        let Transform2D(a) = self;
        let Transform2D(b) = other;

        Transform2D([
            [a[0][0]*b[0][0] + a[0][1]*b[1][0] + a[0][2]*b[2][0],   a[0][0]*b[0][1] + a[0][1]*b[1][1] + a[0][2]*b[2][1],    a[0][0]*b[0][2] + a[0][1]*b[1][2] + a[0][2]*b[2][2]],
            [a[1][0]*b[0][0] + a[1][1]*b[1][0] + a[1][2]*b[2][0],   a[1][0]*b[0][1] + a[1][1]*b[1][1] + a[1][2]*b[2][1],    a[1][0]*b[0][2] + a[1][1]*b[1][2] + a[1][2]*b[2][2]],
            [a[2][0]*b[0][0] + a[2][1]*b[1][0] + a[2][2]*b[2][0],   a[2][0]*b[0][1] + a[2][1]*b[1][1] + a[2][2]*b[2][1],    a[2][0]*b[0][2] + a[2][1]*b[1][2] + a[2][2]*b[2][2]],
        ])
    }
}

impl From<SpriteTransform> for Transform2D {
    fn from(sprite_transform: SpriteTransform) -> Transform2D {
        match sprite_transform {
            SpriteTransform::Identity               => Transform2D::identity(),
            SpriteTransform::Translate(x, y)        => Transform2D::translate(x, y),
            SpriteTransform::Scale(x, y)            => Transform2D::scale(x, y),
            SpriteTransform::Rotate(degrees)        => Transform2D::rotate(degrees / 180.0 * f32::consts::PI),
            SpriteTransform::Transform2D(transform) => transform
        }
    }
}