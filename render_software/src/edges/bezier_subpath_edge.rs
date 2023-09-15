use crate::edgeplan::*;

use flo_canvas::curves::bezier::path::*;
use flo_canvas::curves::geo::*;
use flo_canvas::curves::bezier::*;

use smallvec::*;

use std::ops::{Range};
use std::vec;

///
/// Bezier subpath that uses the 'non-zero' algorithm to decide whether a point is inside or outside the shape
///
#[derive(Clone)]
pub struct BezierSubpathNonZeroEdge {
    /// The ID of the shape that's inside this subpath
    shape_id: ShapeId,

    /// The subpath definition
    subpath: BezierSubpath,
}

///
/// Bezier subpath that uses the 'even-odd' algorithm to decide whether a point is inside or outside the shape
///
#[derive(Clone)]
pub struct BezierSubpathEvenOddEdge {
    /// The ID of the shape that's inside this subpath
    shape_id: ShapeId,

    /// The subpath definition
    subpath: BezierSubpath,
}

///
/// Represents a closed bezier subpath
///
/// To become an edge, this needs to be combined with a winding rule style and a 
///
#[derive(Clone)]
pub struct BezierSubpath {
    /// The curves within this subpath
    curves: Vec<SubpathCurve>,

    /// The bounding box (x coordinates)
    x_bounds: Range<f64>,

    /// The bounding box (y coordinates)
    y_bounds: Range<f64>,
}

#[derive(Clone)]
struct SubpathCurve {
    /// The y bounding box for this curve
    y_bounds: Range<f64>,

    /// x control points (w1, w2, w3, w4)
    wx: (f64, f64, f64, f64),

    /// y control points (w1, w2, w3, w4)
    wy: (f64, f64, f64, f64),

    /// The y-derivative control points (w1, w2, w3)
    wdy: (f64, f64, f64),
}

///
/// An intercept on a bezier subpath
///
#[derive(Clone, Copy, Debug)]
pub struct BezierSubpathIntercept {
    /// The x position of this intercept
    pub x_pos: f64,

    /// The curve that the intercept belongs to
    pub curve_idx: usize,

    /// The t-value of this intercept
    pub t: f64,
}

impl Geo for BezierSubpath {
    type Point = Coord2;
}

impl SubpathCurve {
    /// Converts this to a 'normal' curve
    fn as_curve(&self) -> Curve<Coord2> {
        Curve::from_points(Coord2(self.wx.0, self.wy.0), (Coord2(self.wx.1, self.wy.1), Coord2(self.wx.2, self.wy.2)), Coord2(self.wx.3, self.wy.3))
    }
}

impl BezierPath for BezierSubpath {
    type PointIter = vec::IntoIter<(Coord2, Coord2, Coord2)>;

    #[inline]
    fn start_point(&self) -> Self::Point {
        Coord2(self.curves[0].wx.0, self.curves[0].wy.0)
    }

    fn points(&self) -> Self::PointIter {
        self.curves.iter()
            .map(|curve| (Coord2(curve.wx.1, curve.wy.1), Coord2(curve.wx.2, curve.wy.2), Coord2(curve.wx.3, curve.wy.3)))
            .collect::<Vec<_>>()
            .into_iter()
    }
}

///
/// A bezier subpath can be used as the target of a bezier path factory
///
impl BezierPathFactory for BezierSubpath {
    fn from_points<FromIter: IntoIterator<Item=(Coord2, Coord2, Coord2)>>(start_point: Coord2, points: FromIter) -> Self {
        // This should be much smaller than a pixel: we exclude very short curves whose control polygon is smaller than this
        const MIN_DISTANCE: f64 = 1e-6;

        let mut curves      = vec![];
        let mut last_point  = start_point;

        let mut min_x       = f64::MAX;
        let mut min_y       = f64::MAX;
        let mut max_x       = f64::MIN;
        let mut max_y       = f64::MIN;

        for (cp1, cp2, end_point) in points {
            if last_point.is_near_to(&end_point, MIN_DISTANCE) && control_polygon_length(&Curve::from_points(last_point, (cp1, cp2), end_point)) <= MIN_DISTANCE {
                // This curve is very short, so we exclude it from the path
                continue;
            }

            // Fetch the w values, and calculate the derivative and bounding box
            let wx          = (last_point.x(), cp1.x(), cp2.x(), end_point.x());
            let wy          = (last_point.y(), cp1.y(), cp2.y(), end_point.y());
            let wdy         = derivative4(wy.0, wy.1, wy.2, wy.3);
            let x_bounds    = bounding_box4::<_, Bounds<f64>>(wy.0, wy.1, wy.2, wy.3);
            let y_bounds    = bounding_box4::<_, Bounds<f64>>(wy.0, wy.1, wy.2, wy.3);

            // Update the min, max coordinates
            min_x = min_x.min(x_bounds.min());
            min_y = min_y.min(y_bounds.min());
            max_x = max_x.max(x_bounds.max());
            max_y = max_y.max(y_bounds.max());

            // Add a new curve
            curves.push(SubpathCurve {
                y_bounds:   y_bounds.min()..y_bounds.max(),
                wx:         wx,
                wy:         wy,
                wdy:        wdy,
            });

            // Update the last point to match the end point of the previous curve section
            last_point = end_point;
        }

        // If a subpath isn't closed, then rays might 'escape'
        debug_assert!(start_point == last_point, "Bezier subpaths must be closed ({}, {} != {}, {})", start_point.x(), start_point.y(), last_point.x(), last_point.y());

        if curves.len() == 0 {
            panic!("Bezier subpaths must have at least one curve in them");
        }

        BezierSubpath {
            curves:     curves,
            x_bounds:   min_x..max_x,
            y_bounds:   min_y..max_y
        }
    }
}

impl BezierSubpath {
    ///
    /// Finds the intercepts on a line of this subpath
    ///
    pub fn intercepts_on_line(&self, y_pos: f64) -> impl Iterator<Item=BezierSubpathIntercept> {
        // How close two intercepts have to be to invoke the 'double intercept' algorithm. This really depends on the precision of `solve_basis_for_t'
        const VERY_CLOSE_X: f64 = 1.0;

        // How short the control polygon needs to be between two points to consider them as the same
        const MIN_CONTROL_POLYGON_LENGTH: f64 = 1e-6;

        // Compute the raw intercepts. These can have double intercepts where two curves meet
        let mut intercepts = self.curves
            .iter()
            .enumerate()
            .filter(|(_idx, curve)| curve.y_bounds.contains(&y_pos))
            .flat_map(|(idx, curve)| solve_basis_for_t(curve.wy.0, curve.wy.1, curve.wy.2, curve.wy.3, y_pos).into_iter()
                .filter(|t| *t >= 0.0 && *t <= 1.0)
                .map(move |t| BezierSubpathIntercept { x_pos: de_casteljau4(t, curve.wx.0, curve.wx.1, curve.wx.2, curve.wx.3), curve_idx: idx, t: t, } ))
            .collect::<SmallVec<[_; 4]>>();

        // Sort the intercepts by x position
        intercepts.sort_unstable_by(|a, b| a.x_pos.total_cmp(&b.x_pos));

        if intercepts.len() > 1 {
            // Detect double intercepts
            // We use numerical methods to solve the intercept points, which is combined with the inherent imprecision of floating point numbers, so double intercepts will
            // not always appear at the same place. So the approach is this: if two intercepts have very close x values, are for the end and start of neighboring curves, and
            // are in the same direction, then count that intercept as just one. It's probably possible to fool this algorithm with a suitably constructed self-intersection shape.
            // TODO: if there are very many very short curve sections we might end up with a whole cluster of intercepts that should count as one (isn't clear how short these
            // need to be).
            let mut intercept_idx = 0;
            while intercept_idx < intercepts.len()-1 {
                // Fetch the two intercepts that we want to check for doubling up
                let mut overlap_idx = intercept_idx + 1;

                while overlap_idx < intercepts.len() && (intercepts[intercept_idx].x_pos - intercepts[overlap_idx].x_pos).abs() <= VERY_CLOSE_X {
                    let prev = &intercepts[intercept_idx];
                    let next = &intercepts[overlap_idx];

                    // TODO: this won't work if the overlap happens at the very start of the cruve
                    if ((prev.curve_idx as isize) - (next.curve_idx as isize)).abs() == 1 {
                        // Two points are very close together
                        let prev_curve      = &self.curves[prev.curve_idx];
                        let next_curve      = &self.curves[next.curve_idx];

                        let prev_tangent_y   = de_casteljau3(prev.t, prev_curve.wdy.0, prev_curve.wdy.1, prev_curve.wdy.2);
                        let prev_normal_x    = -prev_tangent_y;
                        let prev_side        = prev_normal_x.signum();

                        let next_tangent_y   = de_casteljau3(next.t, next_curve.wdy.0, next_curve.wdy.1, next_curve.wdy.2);
                        let next_normal_x    = -next_tangent_y;
                        let next_side        = next_normal_x.signum();

                        // Remove one of the intercepts if these two very close points are crossing the subpath in the same direction
                        if prev_side == next_side {
                            // Two intercepts are on the same side of the curve, on subsequent sections: they are (very probably) the same if the 'control polygon' distance between them is small enough
                            if prev.curve_idx < next.curve_idx {
                                let prev_as_curve   = prev_curve.as_curve();
                                let next_as_curve   = next_curve.as_curve();
                                let prev_section    = prev_as_curve.section(prev.t, 1.0);
                                let next_section    = next_as_curve.section(0.0, next.t);
                                let length          = control_polygon_length(&prev_section) + control_polygon_length(&next_section);

                                if length < MIN_CONTROL_POLYGON_LENGTH || (prev.t >= 1.0 && next.t <= 0.0) {
                                    // Points are very close in terms of curve arc length
                                    intercepts.remove(overlap_idx);
                                } else {
                                    overlap_idx += 1;
                                }
                            } else {
                                let prev_as_curve   = prev_curve.as_curve();
                                let next_as_curve   = next_curve.as_curve();
                                let prev_section    = prev_as_curve.section(0.0, prev.t);
                                let next_section    = next_as_curve.section(next.t, 1.0);
                                let length          = control_polygon_length(&prev_section) + control_polygon_length(&next_section);

                                if length < MIN_CONTROL_POLYGON_LENGTH || (prev.t <= 0.0 && next.t >= 1.0) {
                                    // Points are very close in terms of curve arc length
                                    intercepts.remove(overlap_idx);
                                } else {
                                    overlap_idx += 1;
                                }
                            }
                        } else {
                            overlap_idx += 1;
                        }
                    } else {
                        // Only test neighboring edges
                        overlap_idx += 1;
                    }
                }

                // Try the next intercept
                intercept_idx += 1;
            }
        }

        debug_assert!(intercepts.len()%2 == 0, "\n\nIntercepts should be even, but found {} intercepts - {:?} - on line {:?} for path:\n'{}'\n\n", intercepts.len(), intercepts, y_pos, flo_canvas::curves::debug::bezier_path_to_rust_definition(self));

        // Iterate over the results
        intercepts.into_iter()
    }

    ///
    /// Creates a non-zero edge from this subpath
    ///
    pub fn to_non_zero_edge(self, shape_id: ShapeId) -> BezierSubpathNonZeroEdge {
        BezierSubpathNonZeroEdge {
            shape_id:   shape_id,
            subpath:    self,
        }
    }

    ///
    /// Creates a non-zero edge from this subpath
    ///
    pub fn to_even_odd_edge(self, shape_id: ShapeId) -> BezierSubpathEvenOddEdge {
        BezierSubpathEvenOddEdge {
            shape_id:   shape_id,
            subpath:    self,
        }
    }
}

impl EdgeDescriptor for BezierSubpathEvenOddEdge {
    #[inline]
    fn shape(&self) -> ShapeId { self.shape_id }

    #[inline]
    fn bounding_box(&self) -> ((f64, f64), (f64, f64)) {
        ((self.subpath.x_bounds.start, self.subpath.y_bounds.start), (self.subpath.x_bounds.end, self.subpath.y_bounds.end))
    }

    #[inline]
    fn intercepts(&self, y_positions: &[f64], output: &mut [SmallVec<[(EdgeInterceptDirection, f64); 2]>]) {
        let mut y_pos_iter  = y_positions.iter();
        let mut output_iter = output.iter_mut();

        while let (Some(y_pos), Some(output)) = (y_pos_iter.next(), output_iter.next()) {
            let intercepts = self.subpath.intercepts_on_line(*y_pos);

            if self.subpath.y_bounds.contains(y_pos) {
                *output = intercepts.into_iter()
                    .map(|intercept| (EdgeInterceptDirection::Toggle, intercept.x_pos))
                    .collect();
            } else {
                *output = smallvec![];
            }
        }
    }
}

impl EdgeDescriptor for BezierSubpathNonZeroEdge {
    #[inline]
    fn shape(&self) -> ShapeId { self.shape_id }

    #[inline]
    fn bounding_box(&self) -> ((f64, f64), (f64, f64)) {
        ((self.subpath.x_bounds.start, self.subpath.y_bounds.start), (self.subpath.x_bounds.end, self.subpath.y_bounds.end))
    }

    #[inline]
    fn intercepts(&self, y_positions: &[f64], output: &mut [SmallVec<[(EdgeInterceptDirection, f64); 2]>]) {
        let mut y_pos_iter  = y_positions.iter();
        let mut output_iter = output.iter_mut();

        while let (Some(y_pos), Some(output)) = (y_pos_iter.next(), output_iter.next()) {
            let intercepts = self.subpath.intercepts_on_line(*y_pos);

            if self.subpath.y_bounds.contains(y_pos) {
                *output = intercepts.into_iter()
                    .map(|intercept| {
                        // Compute the direction that the ray is crossing the curve
                        let t               = intercept.t;
                        let (d1, d2, d3)    = self.subpath.curves[intercept.curve_idx].wdy;

                        let tangent_y       = de_casteljau3(t, d1, d2, d3);
                        let normal_x        = -tangent_y;
                        let side            = normal_x.signum();

                        // The basic approach to the normal is to get the dot product like this, but we precalculate just what we need
                        //let normal  = self.curve.normal_at_pos(t);
                        //let side    = (normal.x() * 1.0 + normal.y() * 0.0).signum();  // Dot product with the 'ray' direction of the scanline

                        if side <= 0.0 {
                            (EdgeInterceptDirection::DirectionOut, intercept.x_pos)
                        } else {
                            (EdgeInterceptDirection::DirectionIn, intercept.x_pos)
                        }
                    }).collect();
            } else {
                *output = smallvec![];
            }
        }
    }
}
