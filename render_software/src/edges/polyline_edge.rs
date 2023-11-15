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
    Lines { space: Space1D<PolylineLine>, points: Vec<Coord2> },
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
    polyline:   Polyline,
}

///
/// A path filled with the even-odd winding rule with an edge defined by a polyline
///
#[derive(Clone)]
pub struct PolylineEvenOddEdge {
    shape_id:   ShapeId,
    polyline:   Polyline,
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
            PolylineValue::Empty                    => { }
            PolylineValue::Lines { space, points }  => { self.value = PolylineValue::Lines { space, points } }

            PolylineValue::Points(coords)           => {
                // Calculate the coefficients and y-ranges for all of the lines
                let mut bounds_min = (f64::MAX, f64::MAX);
                let mut bounds_max = (f64::MIN, f64::MIN);

                let lines = coords.iter()
                    .copied()
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
                self.value          = PolylineValue::Lines { space: Space1D::from_data(lines), points: coords };
                self.bounding_box   = (bounds_min, bounds_max);
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

    ///
    /// Returns a transformed version of this polyline
    ///
    /// The result will need to have 'prepare_to_render' called on it after this call
    ///
    pub fn transform_unprepared(&self, transform: &canvas::Transform2D) -> Self {
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

            PolylineValue::Lines { points, .. } => {
                // Transform the original set of points (it is possible to transform the lines except when they're horizontal)
                let points = points.iter().map(|point| transform_coord(point, transform)).collect();

                Self {
                    value:          PolylineValue::Points(points),
                    bounding_box:   self.bounding_box,
                }
            }
        }
    }

    ///
    /// Fills in an intercept list given a list of lines that cross that position
    ///
    #[inline]
    fn fill_intercepts_from_lines<'a>(y_pos: f64, lines: impl Iterator<Item=&'a PolylineLine>, intercepts: &mut SmallVec<[(EdgeInterceptDirection, f64); 2]>) {
        let mut last_direction = EdgeInterceptDirection::Toggle;

        for line in lines {
            let x_pos       = line.x_pos(y_pos);
            let direction   = if let EdgeInterceptDirection::Toggle = line.direction { 
                // TODO: this really requires ordering this intercept according to the other lines 
                // (This happens only on horizontal lines too, so we probably should instead consider the intercept direction to come from the end point of the previous line)
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

    ///
    /// Finds all of the intercepts along a line at a given y-position
    ///
    #[inline]
    pub fn intercepts_on_line(&self, y_pos: f64, intercepts: &mut SmallVec<[(EdgeInterceptDirection, f64); 2]>) {
        if let PolylineValue::Lines { space, .. } = &self.value {
            // All the lines passing through y_pos are included here (as ranges are exclusive, this will exclude the end point of the line)
            Self::fill_intercepts_from_lines(y_pos, space.data_at_point(y_pos), intercepts);
        } else {
            debug_assert!(false, "Tried to get intercepts for a polyline without preparing it");
        }
    }

    ///
    /// Finds all of the intercepts along a line at an ordered set of y positions
    ///
    pub fn intercepts_on_lines(&self, ordered_y_pos: &[f64], intercepts: &mut [SmallVec<[(EdgeInterceptDirection, f64); 2]>]) {
        // We need an epsilon value to ensure the requested range covers all the y values
        const EPSILON: f64 = 1e-8;

        if let PolylineValue::Lines { space, .. } = &self.value {
            // Calculate the range of y values that we'll be processing
            let y_range = ordered_y_pos.iter()
                .map(|y| *y..(y+EPSILON))
                .reduce(|a, b| (a.start.min(b.start))..(a.end.max(b.end)));
            let y_range = if let Some(y_range) = y_range { y_range } else { return; };

            // Fetch all the lines in this range
            let mut line_regions    = space.regions_in_range(y_range);
            let mut current_region  = if let Some(region) = line_regions.next() { region } else { return; };

            for (y_pos, intercepts) in ordered_y_pos.iter().zip(intercepts.iter_mut()) {
                // Move the current range forward until it overlaps this y-position (we rely on the y positions being in ascending order here)
                while current_region.0.end <= *y_pos {
                    current_region = if let Some(region) = line_regions.next() { region } else { return; };
                }

                if current_region.0.start <= *y_pos  {
                    // Fill the intercepts for this y-position
                    Self::fill_intercepts_from_lines(*y_pos, current_region.1.iter().copied(), intercepts);
                }
            }
        } else {
            debug_assert!(false, "Tried to get intercepts for a polyline without preparing it");
        }
    }


    ///
    /// Fills in an intercept list given a list of lines that cross that position, setting the intercept direction to 'toggle'
    ///
    #[inline]
    fn toggle_fill_intercepts_from_lines<'a>(y_pos: f64, lines: impl Iterator<Item=&'a PolylineLine>, intercepts: &mut SmallVec<[(EdgeInterceptDirection, f64); 2]>) {
        for line in lines {
            let x_pos       = line.x_pos(y_pos);
            let direction   = EdgeInterceptDirection::Toggle;

            intercepts.push((direction, x_pos));
        }
    }

    ///
    /// Finds all of the intercepts along a line at an ordered set of y positions, making all the intercept directions 'Toggle' for even-odd rendering
    ///
    pub fn toggle_intercepts_on_lines(&self, ordered_y_pos: &[f64], intercepts: &mut [SmallVec<[(EdgeInterceptDirection, f64); 2]>]) {
        // We need an epsilon value to ensure the requested range covers all the y values
        const EPSILON: f64 = 1e-8;

        if let PolylineValue::Lines { space, .. } = &self.value {
            // Calculate the range of y values that we'll be processing
            let y_range = ordered_y_pos.iter()
                .map(|y| *y..(y+EPSILON))
                .reduce(|a, b| (a.start.min(b.start))..(a.end.max(b.end)));
            let y_range = if let Some(y_range) = y_range { y_range } else { return; };

            // Fetch all the lines in this range
            let mut line_regions    = space.regions_in_range(y_range);
            let mut current_region  = if let Some(region) = line_regions.next() { region } else { return; };

            for (y_pos, intercepts) in ordered_y_pos.iter().zip(intercepts.iter_mut()) {
                // Move the current range forward until it overlaps this y-position (we rely on the y positions being in ascending order here)
                while current_region.0.end <= *y_pos {
                    current_region = if let Some(region) = line_regions.next() { region } else { return; };
                }

                if current_region.0.start <= *y_pos  {
                    // Fill the intercepts for this y-position
                    Self::toggle_fill_intercepts_from_lines(*y_pos, current_region.1.iter().copied(), intercepts);
                }
            }
        } else {
            debug_assert!(false, "Tried to get intercepts for a polyline without preparing it");
        }
    }

    ///
    /// Returns the number of lines in this polyline
    ///
    pub fn len(&self) -> usize {
        match &self.value {
            PolylineValue::Empty                => 0,
            PolylineValue::Points(points)       => points.len(),
            PolylineValue::Lines { points, .. } => points.len(),
        }
    }

    ///
    /// Creates a non-zero edge from this polyline
    ///
    pub fn to_non_zero_edge(self, shape_id: ShapeId) -> PolylineNonZeroEdge {
        PolylineNonZeroEdge {
            shape_id: shape_id,
            polyline: self
        }
    }

    ///
    /// Creates an even-odd edge from this polyline
    ///
    pub fn to_even_odd_edge(self, shape_id: ShapeId) -> PolylineEvenOddEdge {
        PolylineEvenOddEdge {
            shape_id: shape_id,
            polyline: self
        }
    }

    ///
    /// Returns the coordinates of the end points of the lines that make up this polyline
    ///
    #[inline]
    pub fn points<'a>(&'a self) -> impl 'a + Iterator<Item=Coord2> {
        match &self.value {
            PolylineValue::Empty                => panic!("Polyline is empty"),
            PolylineValue::Points(points)       => points.iter().copied(),
            PolylineValue::Lines { points, .. } => points.iter().copied(),
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
            shape_id: shape_id,
            polyline: Polyline::new(points)
        }
    }

    ///
    /// The number of lines in this edge
    ///
    #[inline]
    pub fn len(&self) -> usize {
        self.polyline.len()
    }

    ///
    /// Returns a new polyline edge after a transform
    ///
    pub fn transform_as_self(&self, transform: &canvas::Transform2D) -> Self {
        let mut line = self.polyline.transform_unprepared(transform);
        line.prepare_to_render();

        Self {
            shape_id:   self.shape_id,
            polyline:   line
        }
    }
}

impl EdgeDescriptor for PolylineNonZeroEdge {
    fn clone_as_object(&self) -> Arc<dyn EdgeDescriptor> {
        Arc::new(self.clone())
    }

    #[inline]
    fn prepare_to_render(&mut self) {
        self.polyline.prepare_to_render();
    }

    #[inline]
    fn shape(&self) -> ShapeId { self.shape_id }

    fn bounding_box(&self) -> ((f64, f64), (f64, f64)) {
        self.polyline.bounding_box
    }

    fn transform(&self, transform: &canvas::Transform2D) -> Arc<dyn EdgeDescriptor> {
        Arc::new(self.transform_as_self(transform))
    }

    fn intercepts(&self, y_positions: &[f64], output: &mut [SmallVec<[(EdgeInterceptDirection, f64); 2]>]) {
        self.polyline.intercepts_on_lines(y_positions, output)
    }
}

impl PolylineEvenOddEdge {
    ///
    /// Creates a new non-zero polyline edge
    ///
    #[inline]
    pub fn new(shape_id: ShapeId, points: impl IntoIterator<Item=Coord2>) -> Self {
        Self {
            shape_id: shape_id,
            polyline: Polyline::new(points)
        }
    }

    ///
    /// The number of lines in this edge
    ///
    #[inline]
    pub fn len(&self) -> usize {
        self.polyline.len()
    }

    ///
    /// The number of spatial regions in this polyline
    ///
    pub fn num_regions(&self) -> usize {
        match &self.polyline.value {
            PolylineValue::Lines { space, .. } => { space.all_regions().count() }

            _ => 0,
        }
    }

    ///
    /// Returns a new polyline edge after a transform
    ///
    pub fn transform_as_self(&self, transform: &canvas::Transform2D) -> Self {
        let mut line = self.polyline.transform_unprepared(transform);
        line.prepare_to_render();

        Self {
            shape_id: self.shape_id,
            polyline: line
        }
    }
}

impl EdgeDescriptor for PolylineEvenOddEdge {
    fn clone_as_object(&self) -> Arc<dyn EdgeDescriptor> {
        Arc::new(self.clone())
    }

    #[inline]
    fn prepare_to_render(&mut self) {
        self.polyline.prepare_to_render();
    }

    #[inline]
    fn shape(&self) -> ShapeId { self.shape_id }

    fn bounding_box(&self) -> ((f64, f64), (f64, f64)) {
        self.polyline.bounding_box
    }

    fn transform(&self, transform: &canvas::Transform2D) -> Arc<dyn EdgeDescriptor> {
        Arc::new(self.transform_as_self(transform))
    }

    fn intercepts(&self, y_positions: &[f64], output: &mut [SmallVec<[(EdgeInterceptDirection, f64); 2]>]) {
        self.polyline.toggle_intercepts_on_lines(y_positions, output);
    }
}
