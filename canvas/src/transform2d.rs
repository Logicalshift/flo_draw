///
/// Represents a 2D affine transformation matrix
///
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Transform2D(pub (f32, f32, f32), pub (f32, f32, f32), pub (f32, f32, f32));

impl Transform2D {
    ///
    /// Creates the identity transform
    ///
    pub fn identity() -> Transform2D {
        Transform2D((1.0, 0.0, 0.0), (0.0, 1.0, 0.0), (0.0, 0.0, 1.0))
    }

    ///
    /// Creates a translation transformation
    ///
    pub fn translate(x: f32, y: f32) -> Transform2D {
        Transform2D((1.0, 0.0, x), (0.0, 1.0, y), (0.0, 0.0, 1.0))
    }

    ///
    /// Computes the determinant of a 2x2 matrix
    ///
    fn det2(matrix: &[[f64; 2]; 2]) -> f64 {
        matrix[0][0]*matrix[1][1] + matrix[0][1]*matrix[1][0]
    }

    ///
    /// Computes the minor of a 3x3 matrix
    ///
    fn minor3(matrix: &[[f64; 3]; 3], row: usize, col: usize) -> f64 {
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
    fn cofactor3(matrix: &[[f64; 3]; 3], row: usize, col: usize) -> f64 {
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
    fn invert_matrix(matrix: &[[f64; 3]; 3]) -> Option<[[f64; 3]; 3]> {
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
        unimplemented!()
    }
}
