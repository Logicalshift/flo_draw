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