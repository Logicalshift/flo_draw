use flo_render_software::draw::*;
use flo_render_software::pixel::*;
use flo_render_software::scanplan::*;

use flo_canvas::*;

///
/// Generates a plan for layer 0 of a drawing at a particular y-position (coordinates are in the -1 to 1 range for a canvas drawing)
///
fn plan_layer_0_line_on_drawing(instructions: impl IntoIterator<Item=Draw>, y_pos: f64) -> ScanlinePlan {
    // Draw to the canvas
    let mut drawing = CanvasDrawing::<F32LinearPixel, 4>::empty();
    drawing.set_pixel_height(1080.0);
    drawing.draw(instructions);

    // We'll try to generate the plan for layer 0
    let edges = drawing.edges_for_layer(LayerId(0)).expect("Expected layer 0 to be generated");

    // Create the planner
    let planner = ShardScanPlanner::default();

    // Request a particular line be scanned (we use a square 1080x1080 region for this)
    let y_positions     = [y_pos];
    let mut scanlines   = [(0.0, ScanlinePlan::default())];
    let transform       = ScanlineTransform::for_region(&(-1.0..1.0), 1080);

    planner.plan_scanlines(edges, &transform, &y_positions, -1.0..1.0, &mut scanlines);

    // Swap out to get the result
    use std::mem;
    let mut result = ScanlinePlan::default();
    mem::swap(&mut scanlines[0].1, &mut result);

    result
}

#[test]
pub fn render_45_degree_triangle() {
    // Read the center line from a triangle with 45-degree edges
    let plan = plan_layer_0_line_on_drawing(vec![
        Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 1.0)),
        Draw::CanvasHeight(1080.0),
        Draw::CenterRegion((-540.0, -540.0), (540.0, 540.0)),
        Draw::Path(PathOp::NewPath),
        Draw::Path(PathOp::Move(-200.0, -100.0)),
        Draw::Path(PathOp::Line(0.0, 100.0)),
        Draw::Path(PathOp::Line(200.0, -100.0)),
        Draw::Path(PathOp::Line(-200.0, -100.0)),
        Draw::FillColor(Color::Rgba(1.0, 1.0, 1.0, 1.0)),
        Draw::Fill
    ], 0.0);

    let spans = plan.spans();

    // Should be three spans (two spans where the triangle partially covers the pixels, and 1 where it fully covers the pixels)
    assert!(spans.len() == 3, "Number of spans != 0 {:?}", plan);

    // Order should be 'transparent, opaque, transparent'
    assert!(!spans[0].is_opaque(), "First span should not be opaque {:?}", plan);
    assert!(spans[1].is_opaque(), "Second span should not be transparent {:?}", plan);
    assert!(!spans[2].is_opaque(), "Third span should not be opaque {:?}", plan);
}