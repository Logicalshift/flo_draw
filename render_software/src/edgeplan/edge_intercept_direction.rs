///
/// Describes the direction of an edge intercept
///
/// * `Toggle` intercepts enter and leave the shape every time an edge is crossed.
/// * `DirectionOut` indicates an edge with the normal facing outwards (increasing the intercept counter).
/// * `DirectionIn` indicates an edge with the normal facing inwards (decreasing the intercept counter).
///
/// `Toggle` can be used to implement the even-odd winding rule, and the `DirectionOut` and `DirectionIn`
/// directions can be used for the non-zero winding rule.
///
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
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
