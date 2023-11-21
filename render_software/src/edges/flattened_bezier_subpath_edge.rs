use super::bezier_subpath_edge::*;
use super::polyline_edge::*;

use crate::edgeplan::*;

use flo_canvas as canvas;
use smallvec::*;

use std::sync::*;

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
                let mut polyline = path.flatten_to_polyline(min_distance, flatness);
                polyline.prepare_to_render();
                self.value = FlattenedBezierSubpathValue::Polyline(polyline);
            },
        }
    }

    ///
    /// Applies a transform to this subpath
    ///
    pub fn transform(&self, transform: &canvas::Transform2D) -> Self {
        match &self.value {
            FlattenedBezierSubpathValue::None => {
                Self { value: FlattenedBezierSubpathValue::None }
            },

            FlattenedBezierSubpathValue::BezierSubPath { path, min_distance, flatness } => {
                let path = path.transform(transform);
                Self { value: FlattenedBezierSubpathValue::BezierSubPath { path: path, min_distance: *min_distance, flatness: *flatness } }
            },

            FlattenedBezierSubpathValue::Polyline(polyline) => {
                let mut polyline = polyline.transform_unprepared(transform);
                polyline.prepare_to_render();
                Self { value: FlattenedBezierSubpathValue::Polyline(polyline) }
            }
        }
    }
}

impl EdgeDescriptor for FlattenedBezierNonZeroEdge {
    fn clone_as_object(&self) -> Arc<dyn EdgeDescriptor> {
        Arc::new(self.clone())
    }

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

    #[inline]
    fn transform(&self, transform: &canvas::Transform2D) -> Arc<dyn EdgeDescriptor> {
        Arc::new(self.transform_as_self(transform))
    }

    fn intercepts(&self, y_positions: &[f64], output: &mut [SmallVec<[EdgeDescriptorIntercept; 2]>]) {
        match &self.path.value {
            FlattenedBezierSubpathValue::Polyline(line) => {
                line.intercepts_on_lines(y_positions, output);
            }

            _ => { debug_assert!(false) }
        }
    }
}

impl FlattenedBezierEvenOddEdge {
    ///
    /// Applies a transform to this edge
    ///
    pub fn transform_as_self(&self, transform: &canvas::Transform2D) -> Self {
        let path            = self.path.transform(transform);
        let mut new_edge    = Self {
            shape_id:   self.shape_id,
            path:       path,
        };

        new_edge.prepare_to_render();
        new_edge
    }
}

impl FlattenedBezierNonZeroEdge {
    ///
    /// Applies a transform to this edge
    ///
    pub fn transform_as_self(&self, transform: &canvas::Transform2D) -> Self {
        let path            = self.path.transform(transform);
        let mut new_edge    = Self {
            shape_id:   self.shape_id,
            path:       path,
        };

        new_edge.prepare_to_render();
        new_edge
    }
}

impl EdgeDescriptor for FlattenedBezierEvenOddEdge {
    fn clone_as_object(&self) -> Arc<dyn EdgeDescriptor> {
        Arc::new(self.clone())
    }

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

    #[inline]
    fn transform(&self, transform: &canvas::Transform2D) -> Arc<dyn EdgeDescriptor> {
        Arc::new(self.transform_as_self(transform))
    }

    fn intercepts(&self, y_positions: &[f64], output: &mut [SmallVec<[EdgeDescriptorIntercept; 2]>]) {
        match &self.path.value {
            FlattenedBezierSubpathValue::Polyline(line) => {
                line.intercepts_on_lines(y_positions, output);

                for intercepts in output.iter_mut() {
                    for EdgeDescriptorIntercept { direction, .. } in intercepts.iter_mut() {
                        *direction = EdgeInterceptDirection::Toggle;
                    }
                }
            }

            _ => { debug_assert!(false) }
        }
    }
}
