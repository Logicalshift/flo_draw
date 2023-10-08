use super::canvas_drawing::*;
use super::drawing_state::*;

use crate::edges::*;
use crate::edgeplan::*;
use crate::pixel::*;

use flo_canvas::curves::bezier::path::*;

use std::sync::*;

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

    ///
    /// Sets the width of the stroke
    ///
    pub fn line_width_pixels(&mut self, pixel_width: f64, height_pixels: f64) {
        let pixel_size  = 2.0/height_pixels;

        self.stroke_width = pixel_size * pixel_width;
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

        // Create the edge
        let stroke_edge = LineStrokeEdge::new(shape_id, current_state.path_edges.clone(), current_state.subpaths.clone(), width, stroke_options);
        current_state.clip_shape(shape_id, vec![stroke_edge]).into_iter()
            .for_each(|edge| current_layer.edges.add_edge(edge));
    }
}
