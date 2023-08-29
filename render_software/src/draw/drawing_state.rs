use crate::pixel::*;
use crate::edgeplan::*;

use flo_canvas as canvas;
use flo_canvas::curves::line::*;
use flo_canvas::curves::bezier::*;

///
/// A brush represents what will be used to fill in the next region 
///
#[derive(Clone)]
pub enum Brush {
    /// Basic solid colour brush (will be drawn opaque so the image behind will be hidden)
    OpaqueSolidColor(canvas::Color),

    /// Transparent solid colour brush (will be blended with the image behind)
    TransparentSolidColor(canvas::Color),
}

///
/// Represents the active drawing state for a canvas drawing
///
#[derive(Clone)]
pub struct DrawingState {
    /// The shape descriptor that will be used for filling the next shape (or None if we haven't allocated data for it yet)
    pub (super) fill_program: Option<ShapeDescriptor>,

    /// The shape descriptor that will be used for filling the stroke of the next shape (or None if we haven't allocated data for it yet)
    pub (super) stroke_program: Option<ShapeDescriptor>,

    /// The brush to select next time fill_program is None
    pub (super) next_fill_brush: Brush,

    /// The brush to select next time stroke_program is None
    pub (super) next_stroke_brush: Brush,

    /// The current position along the path
    pub (super) path_position: Coord2,

    /// The edges of the current path in this drawing state
    pub (super) path_edges: Vec<Curve<Coord2>>,

    /// Indexes of the points where the subpaths starts
    pub (super) subpaths: Vec<usize>,
}

impl Default for DrawingState {
    fn default() -> Self {
        DrawingState { 
            fill_program:       None,
            stroke_program:     None,
            next_fill_brush:    Brush::OpaqueSolidColor(canvas::Color::Rgba(0.0, 0.0, 0.0, 1.0)),
            next_stroke_brush:  Brush::OpaqueSolidColor(canvas::Color::Rgba(0.0, 0.0, 0.0, 1.0)),
            path_position:      Coord2::origin(),
            path_edges:         vec![],
            subpaths:           vec![0],
        }
    }
}

impl DrawingState {
    ///
    /// Updates the state so that the next shape added will use a solid fill colour 
    ///
    pub fn fill_solid_color(&mut self, colour: canvas::Color) {
        // This clears the fill program so we allocate data for it next time
        self.fill_program = None;

        // Choose opaque or transparent for the brush based on the alpha component
        if colour.alpha_component() >= 1.0 {
            self.next_fill_brush = Brush::OpaqueSolidColor(colour);
        } else {
            self.next_fill_brush = Brush::TransparentSolidColor(colour);
        }
    }

    ///
    /// Updates the state so that the next shape added will use a solid fill colour 
    ///
    pub fn stroke_solid_color(&mut self, colour: canvas::Color) {
        // This clears the fill program so we allocate data for it next time
        self.stroke_program = None;

        // Choose opaque or transparent for the brush based on the alpha component
        if colour.alpha_component() >= 1.0 {
            self.next_stroke_brush = Brush::OpaqueSolidColor(colour);
        } else {
            self.next_stroke_brush = Brush::TransparentSolidColor(colour);
        }
    }

    ///
    /// Dispatches a path operation
    ///
    #[inline]
    pub fn path_op(&mut self, path_op: canvas::PathOp) {
        use canvas::PathOp::*;

        match path_op {
            NewPath                                             => self.path_new(),
            Move(x, y)                                          => self.path_move(x as _, y as _),
            Line(x, y)                                          => self.path_line(x as _, y as _),
            BezierCurve(((cp1x, cp1y), (cp2x, cp2y)), (dx, dy)) => self.path_bezier_curve((cp1x as _, cp1y as _), (cp2x as _, cp2y as _), (dx as _, dy as _)),
            ClosePath                                           => self.path_close(),
        }
    }

    ///
    /// Start a new path
    ///
    pub fn path_new(&mut self) {
        self.path_edges.clear();
        self.subpaths.clear();
        self.subpaths.push(0);
    }

    ///
    /// Moves to start a new subpath
    ///
    pub fn path_move(&mut self, x: f64, y: f64) {
        // Start a new subpath if we've generated any new edges
        if self.subpaths.pop() != Some(self.path_edges.len()) {
            self.subpaths.push(self.path_edges.len());
        }

        // Set the 'last position'
        self.path_position = Coord2(x, y);
    }

    ///
    /// Draws a line to a position
    ///
    pub fn path_line(&mut self, x: f64, y: f64) {
        // Create a line from the last position
        let next_pos    = Coord2(x, y);
        let line        = (self.path_position, next_pos);

        // Store as a bezier curve
        self.path_edges.push(line_to_bezier(&line));

        // Update the position
        self.path_position = next_pos;
    }

    ///
    /// Draws a bezier curve to a position
    ///
    pub fn path_bezier_curve(&mut self, cp1: (f64, f64), cp2: (f64, f64), end: (f64, f64)) {
        // Convert the points
        let cp1 = Coord2(cp1.0, cp1.1);
        let cp2 = Coord2(cp2.0, cp2.1);
        let end = Coord2(end.0, end.1);

        // Create a curve
        let curve = Curve::from_points(self.path_position, (cp1, cp2), end);
        self.path_edges.push(curve);

        // Update the last position
        self.path_position = end;
    }

    ///
    /// Closes the current path
    ///
    pub fn path_close(&mut self) {
        // If the path has 0 edges, we can't close it
        if let Some(subpath_idx) = self.subpaths.last().copied() {
            // Are building a subpath (should always be true)
            if subpath_idx < self.path_edges.len() {
                // Subpath has some path components in it
                let start_point = self.path_edges[subpath_idx].start_point();

                // Want to close by drawing a line from the end of last_curve to the subpath start
                if start_point != self.path_position {
                    self.path_line(start_point.x(), start_point.y());
                }
            }
        }
    }
}
