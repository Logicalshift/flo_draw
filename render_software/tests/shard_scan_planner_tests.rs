use flo_render_software::draw::*;
use flo_render_software::edgeplan::*;
use flo_render_software::pixel::*;
use flo_render_software::pixel_programs::*;
use flo_render_software::scanplan::*;

use flo_canvas::*;
use smallvec::*;

use std::ops::{Range};
use std::sync::*;

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

///
/// Returns true if a range starts on a pixel
///
pub fn starts_on_pixel(x_range: &Range<f64>) -> bool {
    let offset = x_range.start - x_range.start.floor();

    offset.abs() < 1e-6
}

///
/// Returns true if a range ends on a pixel
///
pub fn ends_on_pixel(x_range: &Range<f64>) -> bool {
    let offset = x_range.end - x_range.end.floor();

    offset.abs() < 1e-6
}

#[test]
pub fn triangle_45_degrees() {
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
    assert!(spans.len() == 3, "Number of spans != 3 {:?}", plan);

    // Order should be 'transparent, opaque, transparent'
    assert!(!spans[0].is_opaque(), "First span should not be opaque {:?}", plan);
    assert!(spans[1].is_opaque(), "Second span should not be transparent {:?}", plan);
    assert!(!spans[2].is_opaque(), "Third span should not be opaque {:?}", plan);

    // Alpha values should switch sides
    let first_stack = spans[0].programs().collect::<Vec<_>>();
    assert!(first_stack.len() == 3);
    if let PixelProgramPlan::LinearMerge(alpha1, alpha2) = first_stack[2] {
        assert!(alpha1 < alpha2, "First span is not fading up {:?}", plan);
    } else {
        assert!(false, "First span is not blending {:?}", plan);
    }

    let last_stack = spans[2].programs().collect::<Vec<_>>();
    assert!(last_stack.len() == 3);
    if let PixelProgramPlan::LinearMerge(alpha1, alpha2) = last_stack[2] {
        assert!(alpha1 > alpha2, "Last span is not fading down {:?}", plan);
    } else {
        assert!(false, "Last span is not blending {:?}", plan);
    }

    // The ranges should start/end at pixel boundaries
    assert!(ends_on_pixel(&spans[0].x_range()), "First span does not end on pixel (is {:?}), {:?}", spans[0].x_range(), plan);
    assert!(starts_on_pixel(&spans[1].x_range()), "Second span does not start on pixel (is {:?}), {:?}", spans[1].x_range(), plan);
    assert!(ends_on_pixel(&spans[1].x_range()), "Second span does not end on pixel (is {:?}), {:?}", spans[1].x_range(), plan);
    assert!(starts_on_pixel(&spans[1].x_range()), "Third span does not start on pixel (is {:?}), {:?}", spans[2].x_range(), plan);
}

#[test]
pub fn tall_triangle() {
    for y in 300..900 {
        let pix_y = y;
        let y = (y as f64)/1080.0;
        let y = 2.0 * y - 1.0;

        // Read the center line from a triangle with edges with a high angle 
        let plan = plan_layer_0_line_on_drawing(vec![
            Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 1.0)),
            Draw::CanvasHeight(1080.0),
            Draw::CenterRegion((0.0, 0.0), (1080.0, 1080.0)),
            Draw::Path(PathOp::NewPath),
            Draw::Path(PathOp::Move(400.0, 100.0)),
            Draw::Path(PathOp::Line(540.0, 800.0)),
            Draw::Path(PathOp::Line(680.0, 100.0)),
            Draw::FillColor(Color::Rgba(1.0, 1.0, 1.0, 1.0)),
            Draw::Fill
        ], y);

        let spans = plan.spans();

        // Should be three spans (two spans where the triangle partially covers the pixels, and 1 where it fully covers the pixels)
        assert!(spans.len() == 3, "Number of spans != 3 {:?} (y={:?}, pix_y={:?})", plan, y, pix_y);

        // Order should be 'transparent, opaque, transparent'
        assert!(!spans[0].is_opaque(), "First span should not be opaque {:?}", plan);
        assert!(spans[1].is_opaque(), "Second span should not be transparent {:?}", plan);
        assert!(!spans[2].is_opaque(), "Third span should not be opaque {:?}", plan);

        // Alpha values should switch sides
        let first_stack = spans[0].programs().collect::<Vec<_>>();
        assert!(first_stack.len() == 3);
        if let PixelProgramPlan::LinearMerge(alpha1, alpha2) = first_stack[2] {
            assert!(spans[0].x_range().end - spans[0].x_range().start >= 1.0, "First range uses less than a pixel {:?}, y={:?}, pix_y={:?}", plan, y, pix_y);
            assert!(alpha1 <= alpha2, "First span is not fading up {:?}", plan);
            assert!(alpha1 > 0.0 && alpha1 < 1.0, "First span has no alpha {:?}", plan);
        } else {
            assert!(false, "First span is not blending {:?}", plan);
        }

        let last_stack = spans[2].programs().collect::<Vec<_>>();
        assert!(last_stack.len() == 3);
        if let PixelProgramPlan::LinearMerge(alpha1, alpha2) = last_stack[2] {
            assert!(alpha1 >= alpha2, "Last span is not fading down {:?}", plan);
        } else {
            assert!(false, "Last span is not blending {:?}", plan);
        }

        // The ranges should start/end at pixel boundaries
        assert!(ends_on_pixel(&spans[0].x_range()), "First span does not end on pixel (is {:?}), {:?}", spans[0].x_range(), plan);
        assert!(starts_on_pixel(&spans[1].x_range()), "Second span does not start on pixel (is {:?}), {:?}", spans[1].x_range(), plan);
        assert!(ends_on_pixel(&spans[1].x_range()), "Second span does not end on pixel (is {:?}), {:?}", spans[1].x_range(), plan);
        assert!(starts_on_pixel(&spans[1].x_range()), "Third span does not start on pixel (is {:?}), {:?}", spans[2].x_range(), plan);
    }
}

#[test]
fn subpixel_oblique_line() {
    // Read the center line of an oblique line (should cover a quarter of a pixel, sometimes will cover two pixels)
    let plan = plan_layer_0_line_on_drawing(vec![
        Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 1.0)),
        Draw::CanvasHeight(1080.0),
        Draw::CenterRegion((-540.0, -540.0), (540.0, 540.0)),
        Draw::Path(PathOp::NewPath),
        Draw::Path(PathOp::Move(-0.5, -500.0)),
        Draw::Path(PathOp::Line(11.0, 500.0)),
        Draw::Path(PathOp::Line(11.25, 500.0)),
        Draw::Path(PathOp::Line(-0.25, -500.0)),
        Draw::Path(PathOp::Line(-0.5, -500.0)),
        Draw::FillColor(Color::Rgba(1.0, 1.0, 1.0, 1.0)),
        Draw::Fill
    ], 0.0);

    let spans = plan.spans();

    // Should be one span with two blending instructions
    assert!(spans.len() == 1, "Number of spans != 1 {:?}", plan);

    let programs = spans[0].programs().collect::<Box<[_]>>();
    assert!(programs.len() == 5, "Programs: {:?}", programs);
    assert!(programs.iter().filter(|prog| if let PixelProgramPlan::StartBlend = prog { true } else { false }).count() == 2, "Incorrect number of StartBlend instructions: {:?}",programs);
    assert!(programs.iter().filter(|prog| if let PixelProgramPlan::LinearMerge(_, _) = prog { true } else { false }).count() == 2, "Incorrect number of LinearSourecOver instructions: {:?}",programs);
}

#[test]
fn subpixel_vertical_line() {
    // Read the center line of a thin vertical line (should cover quarter of a pixel)
    let plan = plan_layer_0_line_on_drawing(vec![
        Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 1.0)),
        Draw::CanvasHeight(1080.0),
        Draw::CenterRegion((-540.0, -540.0), (540.0, 540.0)),
        Draw::Path(PathOp::NewPath),
        Draw::Path(PathOp::Move(-0.5, -500.0)),
        Draw::Path(PathOp::Line(-0.5, 500.0)),
        Draw::Path(PathOp::Line(-0.25, 500.0)),
        Draw::Path(PathOp::Line(-0.25, -500.0)),
        Draw::Path(PathOp::Line(-0.5, -500.0)),
        Draw::FillColor(Color::Rgba(1.0, 1.0, 1.0, 1.0)),
        Draw::Fill
    ], 0.0);

    let spans = plan.spans();

    // Should be one span with two blending instructions
    assert!(spans.len() == 1, "Number of spans != 1 {:?}", plan);

    let programs = spans[0].programs().collect::<Box<[_]>>();
    assert!(programs.len() == 5, "Programs: {:?}", programs);
    assert!(programs.iter().filter(|prog| if let PixelProgramPlan::StartBlend = prog { true } else { false }).count() == 2, "Incorrect number of StartBlend instructions: {:?}",programs);
    assert!(programs.iter().filter(|prog| if let PixelProgramPlan::LinearMerge(_, _) = prog { true } else { false }).count() == 2, "Incorrect number of LinearSourecOver instructions: {:?}",programs);
}

#[test]
fn overlapping_subpixel_ranges() {
    // Try planning a concave shape that will force the spans to overlap (by switching direction on a subpixel)
    let y_pos = 400.0;
    let plan = plan_layer_0_line_on_drawing(vec![
        Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 1.0)),
        Draw::CanvasHeight(1080.0),
        Draw::CenterRegion((0.0, 0.0), (1080.0, 1080.0)),
        Draw::Path(PathOp::NewPath),
        Draw::Path(PathOp::Move(99.9, y_pos - 1.0)),
        Draw::Path(PathOp::Line(109.9, y_pos - 1.0)),
        Draw::Path(PathOp::Line(111.5, y_pos + 0.6)),
        Draw::Path(PathOp::Line(115.1, y_pos - 1.0)),
        Draw::Path(PathOp::Line(130.1, y_pos - 1.0)),
        Draw::Path(PathOp::Line(130.1, y_pos + 100.0)),
        Draw::Path(PathOp::Line(99.9, y_pos + 100.0)),
        Draw::FillColor(Color::Rgba(1.0, 1.0, 1.0, 1.0)),
        Draw::Fill
    ], (1080.0-y_pos as f64)/540.0 - 1.0);

    let spans = plan.spans();

    // Should be 7 spans (3 for each intercept, then another where they overlap)
    assert!(spans.len() == 7, "Expected 7 spans, got {}: {:?}", spans.len(), spans);

    // One of these ranges should cover both entering and leaving (so be two intercepts)
    assert!(spans.iter().filter(|span| span.programs().count() >= 5).next().is_some(), "Expected an overlapping span (got {:?})", spans);
}

#[test]
fn vertical_multisampling_creates_solid_rendering() {
    // Create an edge plan that forces multi-sampling
    #[derive(Clone)]
    struct TestEdge(ShapeId);
    impl EdgeDescriptor for TestEdge {
        fn clone_as_object(&self) -> Arc<dyn EdgeDescriptor> {
            Arc::new(TestEdge(self.0))
        }

        fn prepare_to_render(&mut self) {
        }

        fn transform(&self, _transform: &flo_canvas::Transform2D) -> Arc<dyn EdgeDescriptor> {
            Arc::new(TestEdge(self.0))
        }

        fn shape(&self) -> ShapeId {
            self.0
        }

        fn bounding_box(&self) -> ((f64, f64), (f64, f64)) {
            ((0.0, 0.0), (1000.0, 1000.0))
        }

        fn intercepts(&self, y_positions: &[f64], output: &mut [Vec<EdgeDescriptorIntercept>]) {
            // Intercepts are on solid pixel boundaries
            output.iter_mut()
                .zip(y_positions.iter())
                .for_each(|(output, y_pos)| {
                    // We leave the line blank in the middle of the 'apexes' so the scan plan will be different there (effectively a vetical subpixel)
                    if *y_pos != 10.5 {
                        output.extend(vec![
                            // We draw at a slight angle here, the start and end pixels should both be 50% covered after rendering
                            EdgeDescriptorIntercept {
                                x_pos:      10.0 + y_pos,
                                direction:  EdgeInterceptDirection::DirectionIn,
                                position:   EdgePosition(0, 0, 0.0),
                            },
                            EdgeDescriptorIntercept {
                                x_pos:      20.0 + y_pos,
                                direction:  EdgeInterceptDirection::DirectionOut,
                                position:   EdgePosition(0, 0, 1.0),
                            }
                        ])
                    }
                })
        }

        fn apexes(&self, output: &mut Vec<f64>) {
            // We create a bunch of apexes between 10.0 and 11.0 (so we force a multisample there)
            output.extend(vec![10.0, 10.1, 10.4, 10.7, 10.9, 11.0, 11.1])
        }
    }

    // Create an edge plan with this shape in it
    let shape_id            = ShapeId::new();
    let transform           = ScanlineTransform::for_region(&(0.0..1000.0), 1000);
    let mut program_cache   = PixelProgramCache::empty();
    let mut data_cache      = program_cache.create_data_cache();
    let solid_color         = program_cache.add_pixel_program(SolidColorProgram::default());
    let background_color    = program_cache.store_program_data(&solid_color, &mut data_cache, SolidColorData(F32LinearPixel::from_components([0.1, 0.2, 0.3, 1.0])));
    let mut edgeplan        = EdgePlan::new().with_shape(shape_id, ShapeDescriptor { programs: smallvec![background_color], is_opaque: false, z_index: 0 }, vec![TestEdge(shape_id)]);

    // Check that the scan planner produces a multisampling scanline here
    let scan_planner    = ShardScanPlanner::default();
    let mut scanlines   = vec![Default::default(); 4];
    edgeplan.prepare_to_render();
    scan_planner.plan_scanlines(&edgeplan, &transform, &[9.5, 10.5, 11.5, 15.5], 0.0..1000.0, &mut scanlines);

    // For the purposes of the test, we don't really care that
    let before_apexes   = &scanlines[0];
    let with_apexes     = &scanlines[1];
    let after_apexes    = &scanlines[2];

    assert!(before_apexes.0 == 9.5, "before_apexes wrong y pos {:?}", before_apexes);
    assert!(with_apexes.0 == 10.5, "with_apexes wrong y pos {:?}", with_apexes);
    assert!(after_apexes.0 == 11.5, "after_apexes wrong y pos{:?}", after_apexes);

    assert!(before_apexes.1.spans().len() == 3, "Should only be 3 spans before_apexes {:?} (lead-in, actual program, lead-out)", before_apexes);
    assert!(before_apexes.1.spans()[0].programs().count() == 3, "Lead in should be 3 programs before_apexes {:?}", before_apexes.1.spans()[0]);
    assert!(before_apexes.1.spans()[1].programs().count() == 1, "Central span should be one program before_apexes {:?}", before_apexes.1.spans()[1]);

    // We're assuming that the algorithm works a certain way here, the final pixel rendering is all that really matters
    assert!(with_apexes.1.spans().len() == 3, "Should only be 3 spans with_apexes {:?} (lead-in, actual program, lead-out)", with_apexes);
    assert!(with_apexes.1.spans()[0].programs().count() > 3, "Lead in should be >3 programs with_apexes {:?}", with_apexes.1.spans()[0]);
    assert!(with_apexes.1.spans()[1].programs().count() > 1, "Central span should be >1 program with_apexes {:?}", with_apexes.1.spans()[1]);

    assert!(after_apexes.1.spans().len() == 3, "Should only be 3 spans after_apexes {:?} (lead-in, actual program, lead-out)", before_apexes);
    assert!(after_apexes.1.spans()[0].programs().count() == 3, "Lead in should be 3 programs after_apexes {:?}", after_apexes.1.spans()[0]);
    assert!(after_apexes.1.spans()[1].programs().count() == 1, "Central span should be one program after_apexes {:?}", after_apexes.1.spans()[1]);
}
