use crate::edgeplan::*;

use flo_canvas as canvas;
use flo_canvas::curves::geo::*;
use flo_canvas::curves::line::*;

use itertools::*;
use smallvec::*;

use std::ops::{Range};
use std::sync::*;

///
/// A single line within a polyline
///
#[derive(Clone)]
struct PolylineLine {
    /// The y-range covered by this polyline
    y_range: Range<f64>,

    /// The coefficients for this line
    coefficients: LineCoefficients,

    /// The edge intercept direction (which we use as the line moving up or down, or EvenOdd for a horizontal line)
    direction: EdgeInterceptDirection,

    /// The minimum x coordinate (used when a=0)
    min_x: f64,
}

///
/// A polyline is a shape defined by lines joining points together
///
#[derive(Clone)]
enum PolylineValue {
    /// Empty value used to let us 'take' the value of this item
    Empty,

    /// Polyline is represented as a series of points
    Points(Vec<Coord2>),

    /// Polyline is represented as a space divided in the y-axis
    Lines(Space1D<PolylineLine>),
}

///
/// A polyline is a shape defined by lines joining points together
///
#[derive(Clone)]
pub struct Polyline {
    value:          PolylineValue,
    bounding_box:   ((f64, f64), (f64, f64))
}

///
/// A path filled with the non-zero winding rule with an edge defined by a polyline
///
#[derive(Clone)]
pub struct PolylineNonZeroEdge {
    shape_id:   ShapeId,
    line:       Polyline,
}

///
/// A path filled with the even-odd winding rule with an edge defined by a polyline
///
#[derive(Clone)]
pub struct PolylineEvenOddEdge {
    shape_id:   ShapeId,
    line:       Polyline,
}

impl PolylineValue {
    ///
    /// Replaces this with an empty value and returns the result
    ///
    #[inline]
    pub fn take(&mut self) -> PolylineValue {
        use std::mem;

        let mut result = PolylineValue::Empty;
        mem::swap(self, &mut result);

        result
    }
}

impl PolylineLine {
    /// Returns the x position for a y position
    #[inline]
    pub fn x_pos(&self, y: f64) -> f64 {
        let LineCoefficients(a, b, c) = self.coefficients;

        if a == 0.0 {
            self.min_x
        } else {
            (-b*y - c) / a
        }
    }

    ///
    /// Applies a canvas transform to this line
    ///
    #[inline]
    pub fn transform(&self, transform: &canvas::Transform2D) -> Self {
        // Convert the line back to normal coordinates
        let start   = Coord2(self.x_pos(self.y_range.start), self.y_range.start);
        let end     = Coord2(self.x_pos(self.y_range.end), self.y_range.end);

        // Transform these coordinates
        let start   = transform_coord(&start, transform);
        let end     = transform_coord(&end, transform);

        // Recalculate the coefficients, etc
        let coefficients = (start, end).coefficients();

        let y_min = start.y().min(end.y());
        let y_max = start.y().max(end.y());
        let x_min = start.x().min(end.x());

        let direction = if start.y() == end.y() {
            EdgeInterceptDirection::Toggle
        } else if start.y() > end.y() {
            EdgeInterceptDirection::DirectionIn
        } else {
            EdgeInterceptDirection::DirectionOut
        };

        Self {
            y_range:        y_min..y_max,
            coefficients:   coefficients,
            direction:      direction,
            min_x:          x_min,
        }
    }
}

#[inline]
fn transform_coord(point: &canvas::Coord2, transform: &canvas::Transform2D) -> canvas::Coord2 {
    let (x, y) = transform.transform_point(point.x() as _, point.y() as _);

    Coord2(x as _, y as _)
}

impl Polyline {
    ///
    /// Creates a new polyline shape
    ///
    #[inline]
    pub fn new(points: impl IntoIterator<Item=Coord2>) -> Self {
        let mut points = points.into_iter().collect::<Vec<_>>();
        debug_assert!(points.last() == points.get(0), "Polyline is not closed");
        if points.last() != points.get(0) {
            points.push(points.get(0).copied().unwrap());
        }

        Polyline {
            value:          PolylineValue::Points(points),
            bounding_box:   ((0.0, 0.0), (0.0, 0.0)),
        }
    }

    ///
    /// Performs the calculations required to 
    ///
    pub fn prepare_to_render(&mut self) {
        match self.value.take() {
            PolylineValue::Empty        => { }
            PolylineValue::Lines(lines) => { self.value = PolylineValue::Lines(lines) }

            PolylineValue::Points(coords) => {
                // Calculate the coefficients and y-ranges for all of the lines
                let mut bounds_min = (f64::MAX, f64::MAX);
                let mut bounds_max = (f64::MIN, f64::MIN);

                let lines = coords.into_iter()
                    .tuple_windows::<(Coord2, Coord2)>()
                    .map(|line| {
                        // Update bounding box
                        bounds_min.0 = bounds_min.0.min(line.0.x()).min(line.1.x());
                        bounds_min.1 = bounds_min.1.min(line.0.y()).min(line.1.y());
                        bounds_max.0 = bounds_max.0.max(line.0.x()).max(line.1.x());
                        bounds_max.1 = bounds_max.1.max(line.0.y()).max(line.1.y());

                        // Calculate coefficients and coordinates
                        let coefficients    = line.coefficients();
                        let min_x           = line.0.x().min(line.1.x());
                        let min_y           = line.0.y().min(line.1.y());
                        let max_y           = line.0.y().max(line.1.y());

                        let direction       = if line.0.y() == line.1.y() {
                            EdgeInterceptDirection::Toggle
                        } else if line.0.y() > line.1.y() {
                            EdgeInterceptDirection::DirectionIn
                        } else {
                            EdgeInterceptDirection::DirectionOut
                        };

                        // Create the line
                        PolylineLine {
                            y_range:        min_y..max_y,
                            coefficients:   coefficients,
                            direction:      direction,
                            min_x:          min_x,
                        }
                    })
                    .map(|line| (line.y_range.clone(), line));

                // Convert to a 1D space
                self.value          = PolylineValue::Lines(Space1D::from_data(lines));
                self.bounding_box   = (bounds_min, bounds_max);

                assert!(self.is_closed());
            }
        }
    }

    ///
    /// Once `prepare_to_render()` has been called, returns the bounding box of this polyline
    ///
    #[inline]
    pub fn bounding_box(&self) -> ((f64, f64), (f64, f64)) {
        self.bounding_box
    }

    fn is_closed(&self) -> bool {
        match &self.value {
            PolylineValue::Lines(lines) => {
                let all_lines = lines.data().collect::<Vec<_>>();

                let start_y     = all_lines[0].y_range.start;
                let end_y       = all_lines.last().unwrap().y_range.end;
                let start_point = all_lines[0].x_pos(start_y);
                let end_point   = all_lines.last().unwrap().x_pos(end_y);

                assert!((end_y - start_y).abs() < 0.01, "{:?} {:?}", (start_point, start_y), (end_point, end_y));
                assert!((end_point - start_point).abs() < 0.01, "{:?} {:?}", (start_point, start_y), (end_point, end_y));

                (end_y - start_y).abs() < 0.01 && (end_point - start_point).abs() < 0.01
            }

            _ => true,
        }
    }

    ///
    /// Returns a transformed version of this polyline
    ///
    pub fn transform(&self, transform: &canvas::Transform2D) -> Self {
        match &self.value {
            PolylineValue::Empty => Self { value: PolylineValue::Empty, bounding_box: self.bounding_box },

            PolylineValue::Points(points) => {
                let points = points.iter().map(|point| transform_coord(point, transform)).collect();

                // We don't need to transform/recalculate the bounding box as this polyline is not already transformed
                Self {
                    value:          PolylineValue::Points(points),
                    bounding_box:   self.bounding_box,
                }
            }

            PolylineValue::Lines(lines) => {
                // TODO: this is failing (seems to be missing a line?)

                // Recreate the space for the lines
                let lines = lines.data().map(|line| line.transform(transform)).map(|line| (line.y_range.clone(), line));
                let lines = Space1D::from_data(lines);

                // Recalculate the bounding box
                let mut bounds_min = (f64::MAX, f64::MAX);
                let mut bounds_max = (f64::MIN, f64::MIN);

                for line in lines.data() {
                    // Convert to a normal line
                    let start   = Coord2(line.x_pos(line.y_range.start), line.y_range.start);
                    let end     = Coord2(line.x_pos(line.y_range.end), line.y_range.end);
                    let line    = (start, end);

                    // Combine with the bounding box
                    bounds_min.0 = bounds_min.0.min(line.0.x()).min(line.1.x());
                    bounds_min.1 = bounds_min.1.min(line.0.y()).min(line.1.y());
                    bounds_max.0 = bounds_max.0.max(line.0.x()).max(line.1.x());
                    bounds_max.1 = bounds_max.1.max(line.0.y()).max(line.1.y());
                }

                Self {
                    value:          PolylineValue::Lines(lines),
                    bounding_box:   (bounds_min, bounds_max),
                }
            }
        }
    }

    ///
    /// Finds all of the intercepts along this line
    ///
    #[inline]
    pub fn intercepts_on_line(&self, y_pos: f64, intercepts: &mut SmallVec<[(EdgeInterceptDirection, f64); 2]>) {
        if let PolylineValue::Lines(lines) = &self.value {
            // All the lines passing through y_pos are included here (as ranges are exclusive, this will exclude the end point of the line)
            let mut last_direction = EdgeInterceptDirection::Toggle;

            for line in lines.data_at_point(y_pos) {
                let x_pos       = line.x_pos(y_pos);
                let direction   = if let EdgeInterceptDirection::Toggle = line.direction { 
                    match last_direction {
                        EdgeInterceptDirection::DirectionOut    => EdgeInterceptDirection::DirectionIn,
                        EdgeInterceptDirection::DirectionIn     => EdgeInterceptDirection::DirectionOut,
                        EdgeInterceptDirection::Toggle          => EdgeInterceptDirection::Toggle,
                    }
                } else {
                    line.direction
                };

                intercepts.push((direction, x_pos));
                last_direction = direction;
            }
        }
    }

    ///
    /// Returns the number of lines in this polyline
    ///
    pub fn len(&self) -> usize {
        match &self.value {
            PolylineValue::Empty                => 0,
            PolylineValue::Points(points)       => points.len(),
            PolylineValue::Lines(line_space)    => line_space.data().count(),
        }
    }

    ///
    /// Creates a non-zero edge from this polyline
    ///
    pub fn to_non_zero_edge(self, shape_id: ShapeId) -> PolylineNonZeroEdge {
        PolylineNonZeroEdge {
            shape_id: shape_id,
            line: self
        }
    }

    ///
    /// Creates an even-odd edge from this polyline
    ///
    pub fn to_even_odd_edge(self, shape_id: ShapeId) -> PolylineEvenOddEdge {
        PolylineEvenOddEdge {
            shape_id: shape_id,
            line: self
        }
    }
}

impl PolylineNonZeroEdge {
    ///
    /// Creates a new non-zero polyline edge
    ///
    #[inline]
    pub fn new(shape_id: ShapeId, points: impl IntoIterator<Item=Coord2>) -> Self {
        Self {
            shape_id:   shape_id,
            line:       Polyline::new(points)
        }
    }

    ///
    /// The number of lines in this edge
    ///
    #[inline]
    pub fn len(&self) -> usize {
        self.line.len()
    }

    ///
    /// Returns a new polyline edge after a transform
    ///
    pub fn transform_as_self(&self, transform: &canvas::Transform2D) -> Self {
        let line = self.line.transform(transform);

        Self {
            shape_id:   self.shape_id,
            line:       line
        }
    }
}

impl EdgeDescriptor for PolylineNonZeroEdge {
    fn clone_as_object(&self) -> Arc<dyn EdgeDescriptor> {
        Arc::new(self.clone())
    }

    #[inline]
    fn prepare_to_render(&mut self) {
        self.line.prepare_to_render();
    }

    #[inline]
    fn shape(&self) -> ShapeId { self.shape_id }

    fn bounding_box(&self) -> ((f64, f64), (f64, f64)) {
        self.line.bounding_box
    }

    fn transform(&self, transform: &canvas::Transform2D) -> Arc<dyn EdgeDescriptor> {
        let mut line = self.line.transform(transform);
        line.prepare_to_render();

        Arc::new(Self {
            shape_id:   self.shape_id,
            line:       line
        })
    }

    fn intercepts(&self, y_positions: &[f64], output: &mut [SmallVec<[(EdgeInterceptDirection, f64); 2]>]) {
        for (y_pos, output) in y_positions.iter().zip(output.iter_mut()) {
            self.line.intercepts_on_line(*y_pos, output);

            debug_assert!(output.len() % 2 == 0, "Odd number of intercepts on line y={} ({:?})", y_pos, output);
        }
    }
}

impl PolylineEvenOddEdge {
    ///
    /// Creates a new non-zero polyline edge
    ///
    #[inline]
    pub fn new(shape_id: ShapeId, points: impl IntoIterator<Item=Coord2>) -> Self {
        Self {
            shape_id:   shape_id,
            line:       Polyline::new(points)
        }
    }

    ///
    /// The number of lines in this edge
    ///
    #[inline]
    pub fn len(&self) -> usize {
        self.line.len()
    }

    ///
    /// Returns a new polyline edge after a transform
    ///
    pub fn transform_as_self(&self, transform: &canvas::Transform2D) -> Self {
        let line = self.line.transform(transform);

        Self {
            shape_id:   self.shape_id,
            line:       line
        }
    }
}

impl EdgeDescriptor for PolylineEvenOddEdge {
    fn clone_as_object(&self) -> Arc<dyn EdgeDescriptor> {
        Arc::new(self.clone())
    }

    #[inline]
    fn prepare_to_render(&mut self) {
        self.line.prepare_to_render();
    }

    #[inline]
    fn shape(&self) -> ShapeId { self.shape_id }

    fn bounding_box(&self) -> ((f64, f64), (f64, f64)) {
        self.line.bounding_box
    }

    fn transform(&self, transform: &canvas::Transform2D) -> Arc<dyn EdgeDescriptor> {
        let mut line = self.line.transform(transform);
        line.prepare_to_render();

        Arc::new(Self {
            shape_id:   self.shape_id,
            line:       line
        })
    }

    fn intercepts(&self, y_positions: &[f64], output: &mut [SmallVec<[(EdgeInterceptDirection, f64); 2]>]) {
        for (y_pos, output) in y_positions.iter().zip(output.iter_mut()) {
            self.line.intercepts_on_line(*y_pos, output);

            debug_assert!(output.len() % 2 == 0, "Odd number of intercepts on line y={} ({:?})", y_pos, output);

            for (direction, _) in output.iter_mut() {
                *direction = EdgeInterceptDirection::Toggle;
            }
        }
    }
}
