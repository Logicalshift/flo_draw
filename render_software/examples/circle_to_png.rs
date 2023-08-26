use flo_render_software::edgeplan::*;
use flo_render_software::edges::*;
use flo_render_software::pixel::*;
use flo_render_software::render::*;
use flo_render_software::curves::bezier::vectorize::{CircularDistanceField};

use flo_render_software::scanplan::PixelScanPlanner;
use smallvec::*;

///
/// Renders a circle to circle.png using an edge plan and a straightforward pixel program
///
pub fn main() {
    // Create a circular edge (using the circle distance field from flo_curves)
    // This is a more low-level way to represent a 2D scene than describing the rendering instructions using flo_canvas::Draw
    let radius          = 300.0;
    let circle_shape    = ShapeId::new();
    let circle          = ContourEdge::new((960.0-radius, 540.0-radius), circle_shape, CircularDistanceField::with_radius(radius));

    // Create an edge plan that renders this circle
    let edge_plan       = EdgePlan::new()
        .with_shape_description(circle_shape, ShapeDescriptor { programs: smallvec![PixelProgramDataId(1)], is_opaque: true, z_index: 1 })
        .with_edge(circle);

    // Pixel program that renders everything in blue
    let pixel_programs = BasicPixelProgramRunner::from(|_program_id, data: &mut [F32LinearPixel], range, _ypos| {
        let col = F32LinearPixel::from_components([0.0, 0.0, 255.0, 255.0]);
        for x in range {
            data[x as usize] = col;
        }
    });

    // Render to the terminal
    let mut term_renderer = TerminalRenderTarget::new(1920, 1080);

    render_frame_with_planner(PixelScanPlanner::default(), pixel_programs, &edge_plan, &mut term_renderer);
}
