///
/// Represents an OpenGL transformation matrix
///
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Matrix(pub [[f32; 4]; 4]);

impl Matrix {
    ///
    /// Returns the identity matrix
    ///
    pub fn identity() -> Matrix {
        Matrix([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0]
        ])
    }

    ///
    /// Multiplies this matrix with another one
    ///
    pub fn multiply(self, b: Matrix) -> Matrix {
        let Matrix(a)   = self;
        let Matrix(b)   = b;

        let mut res     = [[0.0; 4]; 4];

        for row in 0..4 {
            for col in 0..4 {
                for pos in 0..4 {
                    res[row][col] += a[row][pos] * b[pos][col];
                }
            }
        }

        Matrix(res)
    }

    ///
    /// Converts this matrix to an OpenGL matrix
    ///
    pub fn to_opengl_matrix(&self) -> [f32; 16] {
        let Matrix(matrix) = self;

        [
            matrix[0][0], matrix[0][1], matrix[0][2], matrix[0][3],
            matrix[1][0], matrix[1][1], matrix[1][2], matrix[1][3],
            matrix[2][0], matrix[2][1], matrix[2][2], matrix[2][3],
            matrix[3][0], matrix[3][1], matrix[3][2], matrix[3][3]
        ]
    }

    ///
    /// Flips the Y-coordinates of this matrix
    ///
    pub fn flip_y(self) -> Matrix {
        let Matrix(mut matrix) = self;

        matrix[1][0] = -matrix[1][0];
        matrix[1][1] = -matrix[1][1];
        matrix[1][2] = -matrix[1][2];
        matrix[1][3] = -matrix[1][3];

        Matrix(matrix)
    }
}
