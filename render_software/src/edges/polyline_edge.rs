use crate::edgeplan::*;

use flo_canvas as canvas;
use flo_canvas::curves::geo::*;
use flo_canvas::curves::line::*;

use itertools::*;

use std::ops::{Range};
use std::sync::*;

///
/// A single line within a polyline
///
#[derive(Clone)]
struct PolylineLine {
    /// The index of this subpath in the shape
    subpath_idx: usize,

    /// The index of this line in the shape
    idx: usize,

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
    Points { subpath_idx: usize, coords: Vec<Coord2> },

    /// Polyline is represented as a space divided in the y-axis
    Lines { subpath_idx: usize, space: Space1D<PolylineLine>, points: Vec<Coord2> },
}

///
/// A polyline is a shape defined by lines joining points together
///
#[derive(Clone)]
pub struct Polyline {
    value:          PolylineValue,
    apexes:         Vec<f64>,
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
    pub fn new(subpath_idx: usize, coords: impl IntoIterator<Item=Coord2>, apexes: impl IntoIterator<Item=f64>) -> Self {
        let mut coords = coords.into_iter().collect::<Vec<_>>();
        debug_assert!(coords.last() == coords.get(0), "Polyline is not closed");
        if coords.last() != coords.get(0) {
            coords.push(coords.get(0).copied().unwrap());
        }

        Polyline {
            value:          PolylineValue::Points { subpath_idx, coords },
            apexes:         apexes.into_iter().collect(),
            bounding_box:   ((0.0, 0.0), (0.0, 0.0)),
        }
    }

    ///
    /// Performs the calculations required to render this polyline
    ///
    pub fn prepare_to_render(&mut self) {
        match self.value.take() {
            PolylineValue::Empty                                => { }
            PolylineValue::Lines { subpath_idx, space, points } => { self.value = PolylineValue::Lines { subpath_idx, space, points } }

            PolylineValue::Points { subpath_idx, coords }       => {
                // Calculate the coefficients and y-ranges for all of the lines
                let mut bounds_min = (f64::MAX, f64::MAX);
                let mut bounds_max = (f64::MIN, f64::MIN);

                let lines = coords.iter()
                    .copied()
                    .tuple_windows::<(Coord2, Coord2)>()
                    .enumerate()
                    .map(|(idx, line)| {
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
                            EdgeInterceptDirection::ToggleIn
                        } else if line.0.y() > line.1.y() {
                            EdgeInterceptDirection::DirectionIn
                        } else {
                            EdgeInterceptDirection::DirectionOut
                        };

                        // Create the line
                        PolylineLine {
                            subpath_idx:    subpath_idx,
                            idx:            idx,
                            y_range:        min_y..max_y,
                            coefficients:   coefficients,
                            direction:      direction,
                            min_x:          min_x,
                        }
                    })
                    .map(|line| (line.y_range.clone(), line));

                // Convert to a 1D space
                self.value          = PolylineValue::Lines { subpath_idx: subpath_idx, space: Space1D::from_data(lines), points: coords };
                self.bounding_box   = (bounds_min, bounds_max);
            }
        }

        // Recalculate the apexes if the list is empty (every shape should have one at the top and bottom at least)
        if self.apexes.is_empty() {
            self.recalculate_apexes();
        }
    }

    ///
    /// Recalculates the apexes in this polyline
    ///
    fn recalculate_apexes(&mut self) {
        // Fetch the coordinates that make up this polyline
        let coords = match &self.value {
            PolylineValue::Empty                    => { return; }
            PolylineValue::Points { coords, .. }    => coords,
            PolylineValue::Lines { points, ..}      => points,
        };

        self.apexes.clear();

        // Assuming the line is closed (points.last() == points[0]), look for anywhere where the direction changes in the y-axis
        let lines       = coords.iter().tuple_windows::<(_, _)>();
        let line_pairs  = lines.tuple_windows::<(_, _)>();

        for (line1, line2) in line_pairs {
            // The sign of the direction indicates which direction the line is moving in
            let line1_direction = line1.0.y() - line1.1.y();
            let line2_direction = line2.0.y() - line2.1.y();

            let line1_direction = if line1_direction == 0.0 { 0.0 } else { line1_direction.signum() };
            let line2_direction = if line2_direction == 0.0 { 0.0 } else { line1_direction.signum() };

            // Apexes occur if the direction changes between the two lines
            if line1_direction != line2_direction {
                self.apexes.push(line1.1.y());
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
            PolylineValue::Empty => Self { value: PolylineValue::Empty, apexes: vec![], bounding_box: self.bounding_box },

            PolylineValue::Points { subpath_idx, coords } => {
                let coords      = coords.iter().map(|point| transform_coord(point, transform)).collect();
                let subpath_idx = *subpath_idx;

                // We don't need to transform/recalculate the bounding box as this polyline is not already transformed
                Self {
                    value:          PolylineValue::Points { subpath_idx, coords },
                    apexes:         vec![],
                    bounding_box:   self.bounding_box,
                }
            }

            PolylineValue::Lines { subpath_idx, points, .. } => {
                // Transform the original set of points (it is possible to transform the lines except when they're horizontal)
                let coords      = points.iter().map(|point| transform_coord(point, transform)).collect();
                let subpath_idx = *subpath_idx;

                Self {
                    value:          PolylineValue::Points { subpath_idx, coords },
                    apexes:         vec![],
                    bounding_box:   self.bounding_box,
                }
            }
        }
    }

    ///
    /// Fills in an intercept list given a list of lines that cross that position
    ///
    #[inline]
    fn fill_intercepts_from_lines<'a>(subpath_idx: usize, y_pos: f64, lines: impl Iterator<Item=&'a PolylineLine>, intercepts: &mut Vec<EdgeDescriptorIntercept>) {
        let mut last_direction = EdgeInterceptDirection::ToggleIn;

        for line in lines {
            let x_pos       = line.x_pos(y_pos);
            let direction   = if matches!(line.direction, EdgeInterceptDirection::ToggleIn | EdgeInterceptDirection::ToggleOut) { 
                // TODO: this really requires ordering this intercept according to the other lines 
                // (This happens only on horizontal lines too, so we probably should instead consider the intercept direction to come from the end point of the previous line)
                match last_direction {
                    EdgeInterceptDirection::DirectionOut    => EdgeInterceptDirection::DirectionIn,
                    EdgeInterceptDirection::DirectionIn     => EdgeInterceptDirection::DirectionOut,
                    EdgeInterceptDirection::ToggleIn        => EdgeInterceptDirection::ToggleIn,
                    EdgeInterceptDirection::ToggleOut       => EdgeInterceptDirection::ToggleOut,
                }
            } else {
                line.direction
            };

            // Line position depends on if the line is moving up or down
            let line_pos = match last_direction {
                EdgeInterceptDirection::DirectionOut    => y_pos-line.y_range.start,
                EdgeInterceptDirection::DirectionIn     => line.y_range.end-y_pos,
                EdgeInterceptDirection::ToggleIn        => y_pos-line.y_range.start,
                EdgeInterceptDirection::ToggleOut       => line.y_range.end-y_pos,
            };

            intercepts.push(EdgeDescriptorIntercept { direction, x_pos, position: EdgePosition(subpath_idx, line.idx, line_pos) });
            last_direction = direction;
        }
    }

    ///
    /// Finds all of the intercepts along a line at a given y-position
    ///
    #[inline]
    pub fn intercepts_on_line(&self, y_pos: f64, intercepts: &mut Vec<EdgeDescriptorIntercept>) {
        if let PolylineValue::Lines { space, .. } = &self.value {
            // All the lines passing through y_pos are included here (as ranges are exclusive, this will exclude the end point of the line)
            Self::fill_intercepts_from_lines(self.subpath_idx(), y_pos, space.data_at_point(y_pos), intercepts);
        } else {
            debug_assert!(false, "Tried to get intercepts for a polyline without preparing it");
        }
    }

    ///
    /// Finds all of the intercepts along a line at an ordered set of y positions
    ///
    pub fn intercepts_on_lines(&self, ordered_y_pos: &[f64], intercepts: &mut [Vec<EdgeDescriptorIntercept>]) {
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
                    Self::fill_intercepts_from_lines(self.subpath_idx(), *y_pos, current_region.1.iter().copied(), intercepts);
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
            PolylineValue::Points{ coords, .. } => coords.len(),
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
            PolylineValue::Points{ coords, .. } => coords.iter().copied(),
            PolylineValue::Lines { points, .. } => points.iter().copied(),
        }
    }

    ///
    /// Returns a description of this polyline
    ///
    pub fn description(&self) -> String {
        match &self.value {
            PolylineValue::Empty                => format!("empty"),
            PolylineValue::Points{ coords, .. } => format!("{:?}", coords),
            PolylineValue::Lines { points, .. } => format!("{:?}", points),
        }
    }

    ///
    /// Returns a description of this polyline
    ///
    #[inline]
    pub fn subpath_idx(&self) -> usize {
        match &self.value {
            PolylineValue::Empty                        => 0,
            PolylineValue::Points { subpath_idx, .. }   => *subpath_idx,
            PolylineValue::Lines { subpath_idx, .. }    => *subpath_idx,
        }
    }
}

impl PolylineNonZeroEdge {
    ///
    /// Creates a new non-zero polyline edge
    ///
    #[inline]
    pub fn new(shape_id: ShapeId, subpath_idx: usize, points: impl IntoIterator<Item=Coord2>, apexes: impl IntoIterator<Item=f64>) -> Self {
        Self {
            shape_id: shape_id,
            polyline: Polyline::new(subpath_idx, points, apexes)
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

    fn intercepts(&self, y_positions: &[f64], output: &mut [Vec<EdgeDescriptorIntercept>]) {
        self.polyline.intercepts_on_lines(y_positions, output)
    }

    fn description(&self) -> String {
        format!("Even-odd polyline {:?}: {}", self.shape_id, self.polyline.description())
    }

    fn apexes(&self, output: &mut Vec<f64>) {
        output.extend(self.polyline.apexes.iter().copied());
    }
}

impl PolylineEvenOddEdge {
    ///
    /// Creates a new non-zero polyline edge
    ///
    #[inline]
    pub fn new(shape_id: ShapeId, subpath_idx: usize, points: impl IntoIterator<Item=Coord2>, apexes: impl IntoIterator<Item=f64>) -> Self {
        Self {
            shape_id: shape_id,
            polyline: Polyline::new(subpath_idx, points, apexes)
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

    fn intercepts(&self, y_positions: &[f64], output: &mut [Vec<EdgeDescriptorIntercept>]) {
        self.polyline.intercepts_on_lines(y_positions, output);

        for intercepts in output.iter_mut() {
            for EdgeDescriptorIntercept { direction, .. } in intercepts.iter_mut() {
                *direction = match *direction {
                    EdgeInterceptDirection::DirectionIn     => EdgeInterceptDirection::ToggleIn,
                    EdgeInterceptDirection::DirectionOut    => EdgeInterceptDirection::ToggleOut,
                    other                                   => other,
                };
            }
        }
    }

    fn description(&self) -> String {
        format!("Even-odd polyline {:?}: {}", self.shape_id, self.polyline.description())
    }

    #[inline]
    fn apexes(&self, output: &mut Vec<f64>) {
        output.extend(self.polyline.apexes.iter().copied());
    }
}
