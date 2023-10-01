use super::bezier_subpath_edge::*;
use super::polyline_edge::*;
use crate::edgeplan::*;

use flo_canvas::curves::bezier::path::*;
use flo_canvas::curves::geo::*;
use flo_canvas::curves::bezier::*;

use smallvec::*;
use itertools::*;

use std::iter;
use std::vec;

// These values are good for 4k rendering when flattening curves
const DETAIL: f64   = 2.0/4000.0;
const FLATNESS: f64 = 2.0/4000.0;

///
/// The edges generated by creating a thick line stroke from a path
///
#[derive(Clone)]
pub struct LineStrokeEdge {
    /// The shape ID of this edge
    shape_id: ShapeId,

    /// The options to use for generating the stroke
    stroke_options: StrokeOptions,

    /// The width of the line that this should generate
    width: f64,

    /// The edges of the current path in this drawing state
    path_edges: Vec<Curve<Coord2>>,

    /// Indexes of the points where the subpaths starts
    subpaths: Vec<usize>,

    /// After being prepared: the bezier path for the line stroke
    bezier_path: Vec<PolylineNonZeroEdge>,
}

impl LineStrokeEdge {
    ///
    /// Creates a new line stroke edge
    ///
    /// Subpaths are the indexes into the `path_edges` list that indicate where the stroke should be divided
    ///
    #[inline]
    pub fn new(shape_id: ShapeId, path_edges: Vec<Curve<Coord2>>, subpaths: Vec<usize>, width: f64, stroke_options: StrokeOptions) -> Self {
        LineStrokeEdge {
            shape_id:       shape_id,
            stroke_options: stroke_options,
            width:          width,
            path_edges:     path_edges,
            subpaths:       subpaths,
            bezier_path:    vec![],
        }
    }
}

impl EdgeDescriptor for LineStrokeEdge {
    fn prepare_to_render(&mut self) {
        self.bezier_path.clear();

        // Create bezier subpaths
        for (start_idx, end_idx) in self.subpaths.iter().copied().chain(iter::once(self.path_edges.len())).tuple_windows() {
            if start_idx >= end_idx { continue; }

            // Use a path builder to create a simple bezier path
            let mut path = BezierPathBuilder::<SimpleBezierPath>::start(self.path_edges[start_idx].start_point());
            for curve in self.path_edges[start_idx..end_idx].iter() {
                path = path.curve_to(curve.control_points(), curve.end_point());
            }

            let path = path.build();

            // Thicken it using the path stroking algorithm
            let stroked_path = stroke_path::<BezierSubpath, _>(&path, self.width, &self.stroke_options);

            // Render this path using the non-zero winding rule
            for subpath in stroked_path.into_iter() {
                self.bezier_path.push(subpath.flatten_to_polyline(DETAIL, FLATNESS).to_non_zero_edge(ShapeId(0)));
            }
        }

        // Prepare the paths we created for rendering
        for path in self.bezier_path.iter_mut() {
            path.prepare_to_render();
        }
    }

    fn shape(&self) -> ShapeId {
        self.shape_id
    }

    fn bounding_box(&self) -> ((f64, f64), (f64, f64)) {
        let (mut min_x, mut min_y)  = (f64::MAX, f64::MAX);
        let (mut max_x, mut max_y)  = (f64::MIN, f64::MIN);

        for path in self.bezier_path.iter() {
            let ((path_min_x, path_min_y), (path_max_x, path_max_y)) = path.bounding_box();

            min_x = min_x.min(path_min_x);
            min_y = min_y.min(path_min_y);
            max_x = max_x.max(path_max_x);
            max_y = max_y.max(path_max_y);
        }

        ((min_x, min_y), (max_x, max_y))
    }

    fn intercepts(&self, y_positions: &[f64], output: &mut [SmallVec<[(EdgeInterceptDirection, f64); 2]>]) {
        match self.bezier_path.len() {
            0 => { }
            1 => { self.bezier_path[0].intercepts(y_positions, output) }

            _ => {
                // Fill the initial set of inputs
                self.bezier_path[0].intercepts(y_positions, output);

                // Also add in the intercepts from the other paths
                let mut tmp_output = vec![smallvec![]; y_positions.len()];

                for path in self.bezier_path.iter().skip(1) {
                    // Get the intercepts for this path
                    path.intercepts(y_positions, &mut tmp_output);

                    // Append to the result
                    for (tmp, output) in tmp_output.iter_mut().zip(output.iter_mut()) {
                        output.extend(tmp.drain(..))
                    }
                }

                // Result must be sorted
                for output in output.iter_mut() {
                    output.sort_by(|(_, a), (_, b)| a.total_cmp(b));
                }
            }
        }
    }
}