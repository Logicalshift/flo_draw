use crate::path::*;

///
/// Definition of a path
///
#[derive(Clone, Debug, PartialEq)]
pub struct CanvasPath {
    operations: Vec<PathOp>,
}

impl Default for CanvasPath {
    fn default() -> Self {
        CanvasPath {
            operations: vec![],
        }
    }
}

impl CanvasPath {
    ///
    /// Adds an operation to this path
    ///
    #[inline]
    pub fn draw(&mut self, op: PathOp) {
        if op == PathOp::NewPath {
            self.operations.clear();
        } else {
            self.operations.push(op);
        }
    }
}
