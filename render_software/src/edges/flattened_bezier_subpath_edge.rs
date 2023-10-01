use super::bezier_subpath_edge::*;
use super::polyline_edge::*;

use crate::edgeplan::*;

use smallvec::*;

#[derive(Clone)]
enum FlattenedBezierSubpathValue {
    /// Empty value, used during preparation
    None,

    /// Is a bezier subpath that hasn't been converted yet, along with the conversion values
    BezierSubPath { path: BezierSubpath, min_distance: f64, flatness: f64 },

    /// Converted polyline
    Polyline(Polyline),
}

///
/// A bezier subpath that will be flattened to a polyline before rendering
///
#[derive(Clone)]
pub struct FlattenedBezierSubpath {
    value: FlattenedBezierSubpathValue
}

///
/// A path filled with the non-zero winding rule with an edge defined by a bezier curve rendered as a polyline
///
#[derive(Clone)]
pub struct FlattenedBezierNonZeroEdge {
    pub (super) shape_id:   ShapeId,
    pub (super) path:       FlattenedBezierSubpath,
}

///
/// A path filled with the even-odd winding rule with an edge defined by a bezier curve rendered as a polyline
///
#[derive(Clone)]
pub struct FlattenedBezierEvenOddEdge {
    pub (super) shape_id:   ShapeId,
    pub (super) path:       FlattenedBezierSubpath,
}

impl FlattenedBezierSubpath {
    ///
    /// Creates a flattened bezier subpath from a bezier subpath
    ///
    #[inline]
    pub fn from_subpath(subpath: BezierSubpath, min_distance: f64, flatness: f64) -> Self {
        FlattenedBezierSubpath { 
            value: FlattenedBezierSubpathValue::BezierSubPath { path: subpath, min_distance: min_distance, flatness: flatness }
        }
    }

    ///
    /// Prepares to render this subpath
    ///
    #[inline]
    pub fn prepare_to_render(&mut self) {
        use std::mem;

        // Take the value from inside this subpath
        let mut value = FlattenedBezierSubpathValue::None;
        mem::swap(&mut self.value, &mut value);

        // Generate the polyline by flattening
        match value {
            FlattenedBezierSubpathValue::None            => {},
            FlattenedBezierSubpathValue::Polyline(line)  => { self.value = FlattenedBezierSubpathValue::Polyline(line); },

            FlattenedBezierSubpathValue::BezierSubPath { path, min_distance, flatness } => {
                self.value = FlattenedBezierSubpathValue::Polyline(path.flatten_to_polyline(min_distance, flatness));
            },
        }
    }
}

impl EdgeDescriptor for FlattenedBezierNonZeroEdge {
    #[inline]
    fn prepare_to_render(&mut self) {
        self.path.prepare_to_render();
    }

    #[inline]
    fn shape(&self) -> ShapeId { self.shape_id }

    fn bounding_box(&self) -> ((f64, f64), (f64, f64)) {
        match &self.path.value {
            FlattenedBezierSubpathValue::Polyline(line) => line.bounding_box(),
            _                                           => { debug_assert!(false); ((f64::MIN, f64::MIN), (f64::MAX, f64::MAX)) },
        }
    }

    fn intercepts(&self, y_positions: &[f64], output: &mut [SmallVec<[(EdgeInterceptDirection, f64); 2]>]) {
        match &self.path.value {
            FlattenedBezierSubpathValue::Polyline(line) => {
                for (y_pos, output) in y_positions.iter().zip(output.iter_mut()) {
                    line.intercepts_on_line(*y_pos, output);

                    debug_assert!(output.len() % 2 == 0, "Odd number of intercepts on line y={} ({:?})", y_pos, output);
                }
            }

            _ => { debug_assert!(false) }
        }
    }
}

impl EdgeDescriptor for FlattenedBezierEvenOddEdge {
    #[inline]
    fn prepare_to_render(&mut self) {
        self.path.prepare_to_render();
    }

    #[inline]
    fn shape(&self) -> ShapeId { self.shape_id }

    fn bounding_box(&self) -> ((f64, f64), (f64, f64)) {
        match &self.path.value {
            FlattenedBezierSubpathValue::Polyline(line) => line.bounding_box(),
            _                                           => ((f64::MIN, f64::MIN), (f64::MAX, f64::MAX)),
        }
    }

    fn intercepts(&self, y_positions: &[f64], output: &mut [SmallVec<[(EdgeInterceptDirection, f64); 2]>]) {
        match &self.path.value {
            FlattenedBezierSubpathValue::Polyline(line) => {
                for (y_pos, output) in y_positions.iter().zip(output.iter_mut()) {
                    line.intercepts_on_line(*y_pos, output);

                    debug_assert!(output.len() % 2 == 0, "Odd number of intercepts on line y={} ({:?})", y_pos, output);

                    for (direction, _) in output.iter_mut() {
                        *direction = EdgeInterceptDirection::Toggle;
                    }
                }
            }

            _ => { debug_assert!(false) }
        }
    }
}
