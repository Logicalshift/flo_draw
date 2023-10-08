use super::canvas_drawing::*;
use super::drawing_state::*;

use crate::edges::*;
use crate::edgeplan::*;
use crate::pixel::*;
use crate::pixel_programs::*;

use flo_canvas as canvas;
use flo_canvas::curves::line::*;
use flo_canvas::curves::bezier::*;

use smallvec::*;
use itertools::*;

use std::sync::*;

impl DrawingState {
    ///
    /// Dispatches a path operation
    ///
    #[inline]
    pub fn path_op(&mut self, path_op: canvas::PathOp) {
        use canvas::PathOp::*;

        match path_op {
            NewPath                                             => self.path_new(),
            Move(x, y)                                          => self.path_move(x, y),
            Line(x, y)                                          => self.path_line(x, y),
            BezierCurve(((cp1x, cp1y), (cp2x, cp2y)), (dx, dy)) => self.path_bezier_curve((cp1x, cp1y), (cp2x, cp2y), (dx, dy)),
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
    pub fn path_move(&mut self, x: f32, y: f32) {
        let (x, y) = self.transform.transform_point(x, y);
        let x = x as f64;
        let y = y as f64;

        // Start a new subpath if we've generated any new edges
        if self.subpaths.last() != Some(&self.path_edges.len()) {
            self.subpaths.push(self.path_edges.len());
        }

        // Set the 'last position'
        self.path_position = Coord2(x, y);
    }

    ///
    /// Draws a line to a position
    ///
    pub fn path_line(&mut self, x: f32, y: f32) {
        let (x, y) = self.transform.transform_point(x, y);
        let x = x as f64;
        let y = y as f64;

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
    pub fn path_bezier_curve(&mut self, cp1: (f32, f32), cp2: (f32, f32), end: (f32, f32)) {
        let cp1 = self.transform.transform_point(cp1.0, cp1.1);
        let cp2 = self.transform.transform_point(cp2.0, cp2.1);
        let end = self.transform.transform_point(end.0, end.1);

        // Convert the points
        let cp1 = Coord2(cp1.0 as _, cp1.1 as _);
        let cp2 = Coord2(cp2.0 as _, cp2.1 as _);
        let end = Coord2(end.0 as _, end.1 as _);

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
                    let line = (self.path_position, start_point);
                    self.path_edges.push(line_to_bezier(&line));
                    self.path_position = start_point;
                }
            }
        }
    }

    ///
    /// Makes a shape from the current set of subpaths
    ///
    #[inline]
    pub fn create_path_shape<TEdge>(&self, make_edge: impl Fn(BezierSubpath) -> TEdge) -> Vec<TEdge> 
    where
        TEdge: EdgeDescriptor
    {
        use std::iter;
        use flo_canvas::curves::bezier::path::*;

        let mut edges = vec![];

        for (start_idx, end_idx) in self.subpaths.iter().copied().chain(iter::once(self.path_edges.len())).tuple_windows() {
            if start_idx >= end_idx { continue; }

            // Use a path builder to create a simple bezier path
            let mut path = BezierPathBuilder::<BezierSubpath>::start(self.path_edges[start_idx].start_point());
            for curve in self.path_edges[start_idx..end_idx].iter() {
                path = path.curve_to(curve.control_points(), curve.end_point());
            }

            // Close if unclosed
            if self.path_edges[start_idx].start_point() != self.path_edges[end_idx-1].end_point() {
                path = path.line_to(self.path_edges[start_idx].start_point());
            }

            // Add to the edges
            let path = path.build();
            edges.push(make_edge(path));
        }

        edges
    }
}

impl<TPixel, const N: usize> CanvasDrawing<TPixel, N>
where
    TPixel: 'static + Send + Sync + Pixel<N>,
{
    ///
    /// Creates a shape descriptor from a brush
    ///
    /// The z-index of this descriptor will be set to 0: this should be updated later on
    ///
    pub (super) fn create_shape_descriptor(&mut self, brush: &Brush) -> ShapeDescriptor
    where
        TPixel: 'static + Send + Sync + Pixel<N>,
    {
        use Brush::*;

        let gamma           = self.gamma;
        let program_cache   = &self.program_cache;
        let data_cache      = &mut self.program_data_cache;

        let descriptor = match brush {
            OpaqueSolidColor(color) => {
                let brush_data = program_cache.program_cache.store_program_data(&program_cache.solid_color, data_cache, SolidColorData(TPixel::from_color(*color, gamma)));

                ShapeDescriptor {
                    programs:   smallvec![brush_data],
                    is_opaque:  true,
                    z_index:    0
                }
            }

            TransparentSolidColor(color) => {
                let brush_data = program_cache.program_cache.store_program_data(&program_cache.source_over_color, data_cache, SolidColorData(TPixel::from_color(*color, gamma)));

                ShapeDescriptor {
                    programs:   smallvec![brush_data],
                    is_opaque:  false,
                    z_index:    0
                }
            }
        };

        descriptor
    }

    ///
    /// Adds the current path as a filled path to the current layer
    ///
    pub (super) fn fill(&mut self) {
        // Fetch or create the fill shape descriptor
        let mut shape_descriptor = if let Some(shape_descriptor) = &mut self.current_state.fill_program {
            shape_descriptor.clone()
        } else {
            let shape_descriptor = self.create_shape_descriptor(&self.current_state.next_fill_brush.clone());
            self.current_state.fill_program = Some(shape_descriptor.clone());

            shape_descriptor
        };

        // Retrieve the current layer
        let layers          = &mut self.layers;
        let current_state   = &mut self.current_state;

        let current_layer = layers.get_mut(self.current_layer.0).unwrap();

        // Retain the programs in the shape descriptor and add them to the layer
        for data_id in shape_descriptor.programs.iter().copied() {
            self.program_data_cache.retain_program_data(data_id);
            current_layer.used_data.push(data_id);
        }

        // Set the z-index for the shape descriptor
        let z_index                 = current_layer.z_index;
        shape_descriptor.z_index    = z_index;
        current_layer.z_index += 1;

        // Write the edges using this program
        let shape_id = ShapeId::new();
        current_layer.edges.declare_shape_description(shape_id, shape_descriptor);

        // Create bezier subpaths
        let edges = match current_state.winding_rule {
            canvas::WindingRule::EvenOdd => current_state.clip_shape(shape_id, current_state.create_path_shape(|path| path.to_flattened_even_odd_edge(shape_id))),
            canvas::WindingRule::NonZero => current_state.clip_shape(shape_id, current_state.create_path_shape(|path| path.to_flattened_non_zero_edge(shape_id))),
        };

        edges.into_iter().for_each(|edge| current_layer.edges.add_edge(edge));
    }

    ///
    /// Sets the clipping path to the current path
    ///
    pub (super) fn set_clipping_path(&mut self) {
        let current_state   = &mut self.current_state;

        // TODO: the two arms here are kind of the same with some minor differences, so extracting a function with the common functionality would make sense
        match current_state.winding_rule {
            canvas::WindingRule::EvenOdd => {
                // Create the clipping shape (shape ID doesn't matter for this)
                let shape_id    = ShapeId::new();
                let mut shape   = current_state.create_path_shape(|path| path.to_flattened_even_odd_edge(shape_id));

                // Turn into a clip region
                shape.iter_mut().for_each(|edge| edge.prepare_to_render());

                current_state.clip_path = match &current_state.clip_path {
                    DrawingClipRegion::None                 => DrawingClipRegion::EvenOdd(Arc::new(ClipRegion::new(shape))),

                    DrawingClipRegion::EvenOdd(old_region)  => {
                        let old_region  = (&**old_region).clone().to_object();
                        let shape       = shape_to_object(shape);
                        let region      = ClipRegion::new(vec![ClippedShapeEdge::new(shape_id, Arc::new(old_region), shape)]);

                        DrawingClipRegion::Nested(Arc::new(region))
                    },
                    DrawingClipRegion::NonZero(old_region)  => {
                        let old_region  = (&**old_region).clone().to_object();
                        let shape       = shape_to_object(shape);
                        let region      = ClipRegion::new(vec![ClippedShapeEdge::new(shape_id, Arc::new(old_region), shape)]);

                        DrawingClipRegion::Nested(Arc::new(region))
                    },
                    DrawingClipRegion::Nested(old_region) => {
                        let old_region  = (&**old_region).clone().to_object();
                        let shape       = shape_to_object(shape);
                        let region      = ClipRegion::new(vec![ClippedShapeEdge::new(shape_id, Arc::new(old_region), shape)]);

                        DrawingClipRegion::Nested(Arc::new(region))
                    }
                }
            }

            canvas::WindingRule::NonZero => {
                // Create the clipping shape (shape ID doesn't matter for this)
                let shape_id    = ShapeId::new();
                let mut shape   = current_state.create_path_shape(|path| path.to_flattened_non_zero_edge(shape_id));

                // Turn into a clip region
                shape.iter_mut().for_each(|edge| edge.prepare_to_render());

                current_state.clip_path = match &current_state.clip_path {
                    DrawingClipRegion::None                 => DrawingClipRegion::NonZero(Arc::new(ClipRegion::new(shape))),
                    
                    DrawingClipRegion::EvenOdd(old_region)  => {
                        let old_region  = (&**old_region).clone().to_object();
                        let shape       = shape_to_object(shape);
                        let region      = ClipRegion::new(vec![ClippedShapeEdge::new(shape_id, Arc::new(old_region), shape)]);

                        DrawingClipRegion::Nested(Arc::new(region))
                    },
                    DrawingClipRegion::NonZero(old_region)  => {
                        let old_region  = (&**old_region).clone().to_object();
                        let shape       = shape_to_object(shape);
                        let region      = ClipRegion::new(vec![ClippedShapeEdge::new(shape_id, Arc::new(old_region), shape)]);

                        DrawingClipRegion::Nested(Arc::new(region))
                    },
                    DrawingClipRegion::Nested(old_region) => {
                        let old_region  = (&**old_region).clone().to_object();
                        let shape       = shape_to_object(shape);
                        let region      = ClipRegion::new(vec![ClippedShapeEdge::new(shape_id, Arc::new(old_region), shape)]);

                        DrawingClipRegion::Nested(Arc::new(region))
                    }
                }
            }
        }
    }

    ///
    /// Removes the current clipping path
    ///
    #[inline]
    pub (super) fn unclip(&mut self) {
        self.current_state.clip_path = DrawingClipRegion::None;
    }
}

fn shape_to_object<TEdge>(shape: Vec<TEdge>) -> Vec<Arc<dyn EdgeDescriptor>>
where
    TEdge: 'static + EdgeDescriptor,
{
    shape.into_iter()
        .map(|edge| { let result: Arc<dyn EdgeDescriptor> = Arc::new(edge); result })
        .collect()
}
