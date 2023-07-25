use super::shape_id::*;

use smallvec::*;

pub enum EdgeInterceptDirection {
    ///
    /// If the LHS of the edge is inside of the shape, the RHS is outside of the shape, and vice versa
    ///
    /// This should not be combined with the `DirectionIn` and `DirectionOut` directons but if it is,
    /// this will set the count to 0 if the count is non-zero or 1 otherwise.
    ///
    Toggle,

    ///
    /// Adds 1 to the intercept count for the shape when passing the edge left-to-right. If the 
    /// intercept count is non-zero after this, then the RHS is inside the shape, otherwise it is
    /// outside.
    ///
    DirectionOut,

    ///
    /// Subtracts 1 from the intercept count for the shape when passing the edge left-to-right. If the 
    /// intercept count is non-zero after this, then the RHS is inside the shape, otherwise it is
    /// outside.
    ///
    DirectionIn,
}

///
/// Describes an edge that 
///
pub trait EdgeDescriptor {
    ///
    /// Returns the ID of the shape that this edge is a boundary for
    ///
    /// Edges represent the boundary between the region outside of this shape and the region inside of it. Shapes may
    /// have additional data associated with them, such as a set of programs to run to generate pixels on the inside
    /// and a z-index to indicate which order to draw the shapes in.
    ///
    fn shape(&self) -> ShapeId;

    ///
    /// The minimum and maximum coordinates where this edge might be found
    ///
    /// This does not have to be 100% accurate, so long as the edge is entirely contained within the bounds
    ///
    fn bounding_box(&self) -> ((f64, f64), (f64, f64));

    ///
    /// Returns the intercepts for this edge at a particular y position
    ///
    /// As a general convention, end points should not be included in edges, as there should be an attached 
    /// edge with a start point at the same position. Apex points, where the following edge moves away in 
    /// the y-axis, also should not be counted as an intercept.
    ///
    fn intercepts(&self, y_pos: f64) -> SmallVec<[(EdgeInterceptDirection, f64); 2]>;
}

impl EdgeDescriptor for Box<dyn EdgeDescriptor> {
    #[inline] fn shape(&self) -> ShapeId                                                        { (**self).shape() }
    #[inline] fn bounding_box(&self) -> ((f64, f64), (f64, f64))                                { (**self).bounding_box() }
    #[inline] fn intercepts(&self, y_pos: f64) -> SmallVec<[(EdgeInterceptDirection, f64); 2]>  { (**self).intercepts(y_pos) }
}
