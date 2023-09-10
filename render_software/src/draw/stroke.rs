use super::canvas_drawing::*;
use super::drawing_state::*;

use crate::edges::*;
use crate::edgeplan::*;
use crate::pixel::*;

use flo_canvas::curves::bezier::*;

use itertools::*;

impl DrawingState {
    ///
    /// Sets the width of the stroke
    ///
    pub fn line_width(&mut self, width: f64) {
        let transform   = &self.transform.0;
        let scale       = (transform[0][0]*transform[0][0] + transform[1][0]*transform[1][0]).sqrt();
        let scale       = scale as f64;

        self.stroke_width = width * scale;
    }
}

impl<TPixel, const N: usize> CanvasDrawing<TPixel, N>
where
    TPixel: 'static + Send + Sync + Pixel<N>,
{
    ///
    /// Creates a stroke of the current path
    ///
    pub (super) fn stroke(&mut self) {
        // Fetch or create the fill shape descriptor
        let mut shape_descriptor = if let Some(shape_descriptor) = &mut self.current_state.stroke_program {
            shape_descriptor.clone()
        } else {
            let shape_descriptor = self.create_shape_descriptor(&&self.current_state.next_stroke_brush.clone());
            self.current_state.stroke_program = Some(shape_descriptor.clone());

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

        // Create the stroke options
        let stroke_options = StrokeOptions::default()
            .with_accuracy(2.0/1000.0)
            .with_min_sample_distance(1.0/1000.0)
            .with_start_cap(current_state.stroke_start_cap)
            .with_end_cap(current_state.stroke_end_cap)
            .with_join(current_state.stroke_join);
        let width = current_state.stroke_width;

        // Create bezier subpaths
        use std::iter;
        use flo_canvas::curves::bezier::path::*;

        for (start_idx, end_idx) in current_state.subpaths.iter().copied().chain(iter::once(current_state.path_edges.len())).tuple_windows() {
            if start_idx >= end_idx { continue; }

            // Use a path builder to create a simple bezier path
            let mut path = BezierPathBuilder::<SimpleBezierPath>::start(current_state.path_edges[start_idx].start_point());
            for curve in current_state.path_edges[start_idx..end_idx].iter() {
                path = path.curve_to(curve.control_points(), curve.end_point());
            }

            let path = path.build();

            // Thicken it using the path stroking algorithm
            let stroked_path = stroke_path::<BezierSubpath, _>(&path, width, &stroke_options);

            // Render this path using the non-zero winding rule
            for subpath in stroked_path.into_iter() {
                current_layer.edges.add_edge(Box::new(subpath.to_non_zero_edge(shape_id)));
            }
        }
    }
}
