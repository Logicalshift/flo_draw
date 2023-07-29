use crate::edgeplan::*;

use flo_canvas::curves::geo::*;
use flo_canvas::curves::bezier::*;

use smallvec::*;

///
/// A shape edge described by a 2D bezier curve, using the non-zero winding rule
///
pub struct NonZeroBezierCurveEdge<TCurve> 
where
    TCurve:         BezierCurve,
    TCurve::Point:  Coordinate + Coordinate2D,
{
    /// The ID of the shape that this contour surrounds
    shape_id: ShapeId,

    /// The curve itself
    curve: TCurve,

    /// The curve's x-coordinate points (w1, w2, w3, w4)
    curve_x: (f64, f64, f64, f64),

    /// The curve's y-coordinate points (w1, w2, w3, w4)
    curve_y: (f64, f64, f64, f64),
}

///
/// A shape edge described by a 2D bezier curve, using the even-odd winding rule
///
pub struct EvenOddBezierCurveEdge<TCurve> 
where
    TCurve:         BezierCurve,
    TCurve::Point:  Coordinate + Coordinate2D,
{
    /// The ID of the shape that this contour surrounds
    shape_id: ShapeId,

    /// The curve itself
    curve: TCurve,

    /// The curve's x-coordinate points (w1, w2, w3, w4)
    curve_x: (f64, f64, f64, f64),

    /// The curve's y-coordinate points (w1, w2, w3, w4)
    curve_y: (f64, f64, f64, f64),
}

impl<TCurve> NonZeroBezierCurveEdge<TCurve>
where
    TCurve:         BezierCurve,
    TCurve::Point:  Coordinate + Coordinate2D,
{
    ///
    /// Creates a new bezier curve edge that will use the non-zero winding rule
    ///
    #[inline]
    pub fn new(shape_id: ShapeId, curve: TCurve) -> Self {
        let (w1, (w2, w3), w4)  = curve.all_points();

        Self {
            shape_id:   shape_id, 
            curve:      curve,
            curve_x:    (w1.x(), w2.x(), w3.x(), w4.x()),
            curve_y:    (w1.y(), w2.y(), w3.y(), w4.y()),
        }
    }
}

impl<TCurve> EvenOddBezierCurveEdge<TCurve>
where
    TCurve:         BezierCurve,
    TCurve::Point:  Coordinate + Coordinate2D,
{
    ///
    /// Creates a new bezier curve edge that will use the even-odd winding rule
    ///
    #[inline]
    pub fn new(shape_id: ShapeId, curve: TCurve) -> Self {
        let (w1, (w2, w3), w4)  = curve.all_points();

        Self {
            shape_id:   shape_id, 
            curve:      curve,
            curve_x:    (w1.x(), w2.x(), w3.x(), w4.x()),
            curve_y:    (w1.y(), w2.y(), w3.y(), w4.y()),
        }
    }
}

impl<TCurve> EdgeDescriptor for NonZeroBezierCurveEdge<TCurve>
where
    TCurve:         BezierCurve,
    TCurve::Point:  Coordinate + Coordinate2D,
{
    #[inline]
    fn shape(&self) -> ShapeId { 
        self.shape_id
    }

    #[inline]
    fn bounding_box(&self) -> ((f64, f64), (f64, f64)) { 
        let bounds = self.curve.bounding_box::<Bounds<_>>();

        let min = bounds.min();
        let max = bounds.max();

        ((min.x(), min.y()), (max.x(), max.y()))
    }

    #[inline]
    fn intercepts(&self, y_pos: f64) -> SmallVec<[(EdgeInterceptDirection, f64); 2]> {
        // Calculate the t-values of the intercepts for the curve
        let intercepts = solve_basis_for_t(self.curve_y.0, self.curve_y.1, self.curve_y.2, self.curve_y.3, y_pos);

        // Calculate the x-positions of the intercepts to generate the final result
        // TODO: can we pre-calculate the bits we need to get the normal at a position (we actually just need the x direction...)
        let (w1, w2, w3, w4) = self.curve_x;
        intercepts.into_iter()
            .map(|t| {
                let pos     = basis(t, w1, w2, w3, w4);
                let normal  = self.curve.normal_at_pos(t);
                let side    = (normal.x() * 1.0 + normal.y() * 0.0).signum();  // Dot product with the 'ray' direction of the scanline

                if side <= 0.0 {
                    (EdgeInterceptDirection::DirectionOut, pos)
                } else {
                    (EdgeInterceptDirection::DirectionIn, pos)
                }
            })
            .collect()
    }
}

impl<TCurve> EdgeDescriptor for EvenOddBezierCurveEdge<TCurve>
where
    TCurve:         BezierCurve,
    TCurve::Point:  Coordinate + Coordinate2D,
{
    #[inline]
    fn shape(&self) -> ShapeId { 
        self.shape_id
    }

    #[inline]
    fn bounding_box(&self) -> ((f64, f64), (f64, f64)) { 
        let bounds = self.curve.bounding_box::<Bounds<_>>();

        let min = bounds.min();
        let max = bounds.max();

        ((min.x(), min.y()), (max.x(), max.y()))
    }

    #[inline]
    fn intercepts(&self, y_pos: f64) -> SmallVec<[(EdgeInterceptDirection, f64); 2]> {
        // Calculate the t-values of the intercepts for the curve
        let intercepts = solve_basis_for_t(self.curve_y.0, self.curve_y.1, self.curve_y.2, self.curve_y.3, y_pos);

        // Calculate the x-positions of the intercepts to generate the final result (the even-odd winding rule always toggles)
        let (w1, w2, w3, w4) = self.curve_x;
        intercepts.into_iter()
            .map(|t| basis(t, w1, w2, w3, w4))
            .map(|pos| (EdgeInterceptDirection::Toggle, pos))
            .collect()
    }
}
