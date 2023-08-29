use super::canvas_drawing::*;
use super::drawing_state::*;

use crate::edges::*;
use crate::edgeplan::*;
use crate::pixel::*;
use crate::pixel_programs::*;

use flo_canvas::curves::line::*;
use flo_canvas::curves::bezier::*;

use smallvec::*;

impl<TPixel, const N: usize> CanvasDrawing<TPixel, N>
where
    TPixel: 'static + Send + Sync + Pixel<N>,
{
    ///
    /// Creates a shape descriptor from a brush
    ///
    /// The z-index of this descriptor will be set to 0: this should be updated later on
    ///
    pub fn create_shape_descriptor(&mut self, brush: &Brush) -> ShapeDescriptor
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
    pub fn fill(&mut self) {
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

        // TODO: add as even-odd or non-zero depending on the current winding rule
        for edge in current_state.path_edges.iter() {
            current_layer.edges.add_edge(Box::new(EvenOddBezierCurveEdge::new(shape_id, edge.clone())));
        }

        // Generate lines for unclosed subpaths
        for subpath_idx in 0..current_state.subpaths.len() {
            // The subpath start and end index (inclusive)
            let start_idx   = current_state.subpaths[subpath_idx];
            let end_idx     = if subpath_idx+1 < current_state.subpaths.len() { current_state.subpaths[subpath_idx+1] } else { current_state.path_edges.len() };

            // Ignore zero-length paths
            if end_idx <= start_idx { continue; }
            let end_idx = end_idx - 1;

            // Get the start and end point of the subpath
            let start_point = current_state.path_edges[start_idx].start_point();
            let end_point   = current_state.path_edges[end_idx].end_point();

            // Add a line edge if they don't match
            // TODO: respect the winding rule
            if start_point != end_point {
                current_layer.edges.add_edge(Box::new(EvenOddBezierCurveEdge::<Curve<Coord2>>::new(shape_id, line_to_bezier(&(end_point, start_point)))));
            }
        }
    }
}
