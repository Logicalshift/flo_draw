use flo_render_software::draw::*;
use flo_render_software::edgeplan::*;
use flo_render_software::pixel::*;
use flo_render_software::pixel_programs::*;
use flo_render_software::render::*;
use flo_render_software::scanplan::*;

use flo_canvas::*;
use smallvec::*;

use std::ops::{Range};
use std::sync::*;

///
/// Generates a plan for layer 0 of a drawing at a particular y-position (coordinates are in the -1 to 1 range for a canvas drawing)
///
fn plan_layer_0_line_on_drawing(instructions: impl IntoIterator<Item=Draw>, y_pos: f64) -> ScanlinePlan {
    plan_layer_0_line_on_drawing_with_height(instructions, y_pos, 1080.0)
}

///
/// Generates a plan for layer 0 of a drawing at a particular y-position (coordinates are in the -1 to 1 range for a canvas drawing)
///
fn plan_layer_0_line_on_drawing_with_height(instructions: impl IntoIterator<Item=Draw>, y_pos: f64, pixel_height: f64) -> ScanlinePlan {
    // Draw to the canvas
    let mut drawing = CanvasDrawing::<F32LinearPixel, 4>::empty();
    drawing.set_pixel_height(pixel_height);
    drawing.draw(instructions);

    // We'll try to generate the plan for layer 0
    let edges = drawing.edges_for_layer(LayerId(0)).expect("Expected layer 0 to be generated");

    // Create the planner
    let planner = ShardScanPlanner::default();

    // Request a particular line be scanned (we use a square 1080x1080 region for this)
    let y_positions     = [y_pos];
    let mut scanlines   = [(0.0, ScanlinePlan::default())];
    let transform       = ScanlineTransform::for_region(&(-1.0..1.0), pixel_height as _);

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
    assert!(programs.len() == 3, "Programs: {:?}", programs);
    assert!(programs.iter().filter(|prog| if let PixelProgramPlan::StartBlend = prog { true } else { false }).count() == 1, "Incorrect number of StartBlend instructions: {:?}",programs);
    assert!(programs.iter().filter(|prog| if let PixelProgramPlan::LinearMerge(_, _) = prog { true } else { false }).count() == 1, "Incorrect number of LinearMerge instructions: {:?}",programs);
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
    assert!(programs.len() == 3, "Programs: {:?}", programs);
    assert!(programs.iter().filter(|prog| if let PixelProgramPlan::StartBlend = prog { true } else { false }).count() == 1, "Incorrect number of StartBlend instructions: {:?}",programs);
    assert!(programs.iter().filter(|prog| if let PixelProgramPlan::LinearMerge(_, _) = prog { true } else { false }).count() == 1, "Incorrect number of LinearMerge instructions: {:?}",programs);
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
}

#[test]
fn multisampling_missing_one_quarter() {
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
                    if *y_pos <= 10.375 || *y_pos >= 10.625 {
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

        fn detail_samples(&self) -> usize { 8 }
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

    // The three lines we're interested in (before, after, with apexes)
    let before_apexes   = &scanlines[0];
    let with_apexes     = &scanlines[1];
    let after_apexes    = &scanlines[2];

    // Try rendering the lines
    let scanline_renderer = ScanlineRenderer::new(data_cache.create_program_runner(PixelSize(2.0/1000.0)));
    let mut pixels = vec![F32LinearPixel::default(); 1000];

    // Normal line
    scanline_renderer.render(&ScanlineRenderRegion { y_pos: 9.5, transform: transform }, &before_apexes.1, &mut pixels);

    assert!(pixels[10+9].alpha_component() == 0.5, "before_apexes initial pixel wrong: {:?}", pixels[10+9]);
    assert!(pixels[11+9].alpha_component() == 1.0, "before_apexes mid pixel wrong: {:?}", pixels[11+9]);
    assert!(pixels[20+9].alpha_component() == 0.5, "before_apexes final pixel wrong: {:?}", pixels[20+9]);

    // Apexes line: 1/4 lines are missing when supersampling so we should get a 25% reduction in brightness
    let mut pixels = vec![F32LinearPixel::default(); 1000];
    scanline_renderer.render(&ScanlineRenderRegion { y_pos: 10.5, transform: transform }, &with_apexes.1, &mut pixels);

    // TODO: could pick better apexes to get a more accurate value here (should be 0.75 but our apexes make it 0.55)
    assert!((pixels[11+10].alpha_component()-0.5).abs() <= 0.1, "with_apexes mid pixel wrong: {:?} {:?}", pixels[11+10], &pixels[10..40]);

    // For the purposes of the test, we don't really care precisely what the plan is, the important part is that the rendering is correct
    // However, we check the plan here anyway to make sure the test still makes sense and we're not missing anything
    assert!(before_apexes.0 == 9.5, "before_apexes wrong y pos {:?}", before_apexes);
    assert!(with_apexes.0 == 10.5, "with_apexes wrong y pos {:?}", with_apexes);
    assert!(after_apexes.0 == 11.5, "after_apexes wrong y pos{:?}", after_apexes);

    assert!(before_apexes.1.spans().len() == 3, "Should only be 3 spans before_apexes {:?} (lead-in, actual program, lead-out)", before_apexes);
    assert!(before_apexes.1.spans()[0].programs().count() == 3, "Lead in should be 3 programs before_apexes {:?}", before_apexes.1.spans()[0]);
    assert!(before_apexes.1.spans()[1].programs().count() == 1, "Central span should be one program before_apexes {:?}", before_apexes.1.spans()[1]);

    // We're assuming that the algorithm works a certain way here, the final pixel rendering is all that really matters
    assert!(with_apexes.1.spans().len() == 3, "Should only be 3 spans with_apexes {:?} (lead-in, actual program, lead-out)", with_apexes);
    assert!(with_apexes.1.spans()[0].programs().count() >= 3, "Lead in should be >=3 programs with_apexes {:?}", with_apexes.1.spans()[0]);
    assert!(with_apexes.1.spans()[1].programs().count() > 1, "Central span should be >1 program with_apexes {:?}", with_apexes.1.spans()[1]);

    assert!(after_apexes.1.spans().len() == 3, "Should only be 3 spans after_apexes {:?} (lead-in, actual program, lead-out)", before_apexes);
    assert!(after_apexes.1.spans()[0].programs().count() == 3, "Lead in should be 3 programs after_apexes {:?}", after_apexes.1.spans()[0]);
    assert!(after_apexes.1.spans()[1].programs().count() == 1, "Central span should be one program after_apexes {:?}", after_apexes.1.spans()[1]);
}

#[test]
fn multisampling_missing_one_half() {
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
                    if *y_pos <= 10.5 || *y_pos >= 11.0 {
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

        fn detail_samples(&self) -> usize { 8 }
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

    // The three lines we're interested in (before, after, with apexes)
    let before_apexes   = &scanlines[0];
    let with_apexes     = &scanlines[1];

    // Try rendering the lines
    let scanline_renderer = ScanlineRenderer::new(data_cache.create_program_runner(PixelSize(2.0/1000.0)));
    let mut pixels = vec![F32LinearPixel::default(); 1000];

    // Normal line
    scanline_renderer.render(&ScanlineRenderRegion { y_pos: 9.5, transform: transform }, &before_apexes.1, &mut pixels);

    assert!(pixels[10+9].alpha_component() == 0.5, "before_apexes initial pixel wrong: {:?}", pixels[10+9]);
    assert!(pixels[11+9].alpha_component() == 1.0, "before_apexes mid pixel wrong: {:?}", pixels[11+9]);
    assert!(pixels[20+9].alpha_component() == 0.5, "before_apexes final pixel wrong: {:?}", pixels[20+9]);

    // Apexes line: 1/4 lines are missing when supersampling so we should get a 25% reduction in brightness
    let mut pixels = vec![F32LinearPixel::default(); 1000];
    scanline_renderer.render(&ScanlineRenderRegion { y_pos: 10.5, transform: transform }, &with_apexes.1, &mut pixels);

    assert!((pixels[11+10].alpha_component()-0.5).abs() <= 0.1, "with_apexes mid pixel wrong: {:?} {:?}", pixels[11+10], &pixels[10..40]);
}

#[test]
fn vertical_partial_overlap() {
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
                .for_each(|(output, _y_pos)| {
                    output.extend(vec![
                        // We just draw everything at a 0.75 position offset, which should create a some 50% pixels
                        EdgeDescriptorIntercept {
                            x_pos:      10.75,
                            direction:  EdgeInterceptDirection::DirectionIn,
                            position:   EdgePosition(0, 0, 0.0),
                        },
                        EdgeDescriptorIntercept {
                            x_pos:      20.75,
                            direction:  EdgeInterceptDirection::DirectionOut,
                            position:   EdgePosition(0, 0, 1.0),
                        }
                    ])
                })
        }

        fn apexes(&self, _output: &mut Vec<f64>) {
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
    let mut scanlines   = vec![Default::default()];
    edgeplan.prepare_to_render();
    scan_planner.plan_scanlines(&edgeplan, &transform, &[10.5], 0.0..1000.0, &mut scanlines);

    // The three lines we're interested in (before, after, with apexes)
    let scanline = &scanlines[0];

    // Try rendering the scanline (they're all the same with this layout)
    let scanline_renderer = ScanlineRenderer::new(data_cache.create_program_runner(PixelSize(2.0/1000.0)));
    let mut pixels = vec![F32LinearPixel::default(); 1000];

    scanline_renderer.render(&ScanlineRenderRegion { y_pos: 9.5, transform: transform }, &scanline.1, &mut pixels);

    // Mid pixel should be filled
    assert!(pixels[11].alpha_component() == 1.0, "mid pixel wrong: {:?}", pixels[11]);

    // The edge pixels should both be filled to 50% as they have a 50% vertical overlap
    assert!((pixels[10].alpha_component()-0.25).abs() < 1e-6, "initial pixel wrong: {:?}", pixels[10]);
    assert!((pixels[20].alpha_component()-0.75).abs() < 1e-6, "final pixel wrong: {:?}", pixels[20]);
}

#[test]
fn diagonal_partial_overlap() {
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
                    output.extend(vec![
                        // This creates a diagonal intercept starting at 0.25 of the way along and moving up across two pixels
                        EdgeDescriptorIntercept {
                            x_pos:      10.25 + y_pos * 2.0,
                            direction:  EdgeInterceptDirection::DirectionIn,
                            position:   EdgePosition(0, 0, 0.0),
                        },
                        EdgeDescriptorIntercept {
                            x_pos:      20.75 + y_pos * 2.0,
                            direction:  EdgeInterceptDirection::DirectionOut,
                            position:   EdgePosition(0, 0, 1.0),
                        }
                    ])
                })
        }

        fn apexes(&self, _output: &mut Vec<f64>) {
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
    let mut scanlines   = vec![Default::default()];
    edgeplan.prepare_to_render();
    scan_planner.plan_scanlines(&edgeplan, &transform, &[10.5], 0.0..1000.0, &mut scanlines);

    // The three lines we're interested in (before, after, with apexes)
    let scanline = &scanlines[0];

    // Try rendering the scanline (they're all the same with this layout)
    let scanline_renderer = ScanlineRenderer::new(data_cache.create_program_runner(PixelSize(2.0/1000.0)));
    let mut pixels = vec![F32LinearPixel::default(); 1000];

    scanline_renderer.render(&ScanlineRenderRegion { y_pos: 9.5, transform: transform }, &scanline.1, &mut pixels);
    println!("{:?}", &pixels[0..40]);

    // Mid pixel should be filled (we need to add 9*2 to account for the y positioning as we're reading at y=9)
    assert!(pixels[13 + 10*2].alpha_component() == 1.0, "mid pixel wrong: {:?}", pixels[13 + 9*2]);

    // The edge pixels should both be filled to 50% as they have a 50% vertical overlap
    // 1st pixel: triangle from x=0.25 to y=0.375
    assert!(pixels[10 + 10*2].alpha_component() == 0.75 * 0.375 * 0.5, "1st pixel wrong: {:?}", pixels[10 + 10*2]);

    // 2nd pixel: divider between y=0.375 and y=0.875 (plus fill in between 0-0.375)
    // TODO: reason this is wrong is the linear merge: it is linear between pixel boundaries, but the fade here is not, so the
    // intermediate values are slightly off as a result
    //
    // Correct, by calculating the covered area
    // assert!(pixels[11 + 10*2].alpha_component() == (1.0*(0.875-0.375)*0.5) + 0.375, "2nd pixel wrong: {:?}", pixels[11 + 10*2]);

    // Current: average of the first and last pixel (correct only if 0 and 1 are pixel aligned). Incorrect approximation
    assert!(pixels[11 + 10*2].alpha_component() == (0.984375 + 0.140625)/2.0, "2nd pixel has unexpected value: {:?}", pixels[11 + 10*2]);

    // 3rd pixel: from x=0, y=0.875 to x=0.25, y=1.0
    assert!(pixels[12 + 10*2].alpha_component() == (((1.0-0.875)*0.25)*0.5) + (0.875*0.25) + (0.75 * 1.0), "3rd pixel wrong: {:?}", pixels[12 + 10*2]);
}

#[test]
fn diagonal_full_overlap() {
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
                    output.extend(vec![
                        // This creates a diagonal intercept starting at 0.25 of the way along and moving up across two pixels
                        EdgeDescriptorIntercept {
                            x_pos:      10.0 + y_pos * 2.0,
                            direction:  EdgeInterceptDirection::DirectionIn,
                            position:   EdgePosition(0, 0, 0.0),
                        },
                        EdgeDescriptorIntercept {
                            x_pos:      20.0 + y_pos * 2.0,
                            direction:  EdgeInterceptDirection::DirectionOut,
                            position:   EdgePosition(0, 0, 1.0),
                        }
                    ])
                })
        }

        fn apexes(&self, _output: &mut Vec<f64>) {
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
    let mut scanlines   = vec![Default::default()];
    edgeplan.prepare_to_render();
    scan_planner.plan_scanlines(&edgeplan, &transform, &[10.5], 0.0..1000.0, &mut scanlines);

    // The three lines we're interested in (before, after, with apexes)
    let scanline = &scanlines[0];

    // Try rendering the scanline (they're all the same with this layout)
    let scanline_renderer = ScanlineRenderer::new(data_cache.create_program_runner(PixelSize(2.0/1000.0)));
    let mut pixels = vec![F32LinearPixel::default(); 1000];

    scanline_renderer.render(&ScanlineRenderRegion { y_pos: 9.5, transform: transform }, &scanline.1, &mut pixels);
    println!("{:?}", &pixels[0..40]);

    // Mid pixel should be filled (we need to add 9*2 to account for the y positioning as we're reading at y=9)
    assert!(pixels[13 + 10*2].alpha_component() == 1.0, "mid pixel wrong: {:?}", pixels[13 + 10*2]);

    // The edge pixels should both be filled to 50% as they have a 50% vertical overlap
    assert!(pixels[10 + 10*2].alpha_component() == 0.25, "1st pixel wrong: {:?}", pixels[10 + 10*2]);
    assert!(pixels[11 + 10*2].alpha_component() == 0.75, "2nd pixel wrong: {:?}", pixels[11 + 10*2]);
}

#[test]
fn diagonal_full_overlap_thirds() {
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
                    output.extend(vec![
                        // This creates a diagonal intercept starting at 0.25 of the way along and moving up across two pixels
                        EdgeDescriptorIntercept {
                            x_pos:      10.0 + y_pos * 3.0,
                            direction:  EdgeInterceptDirection::DirectionIn,
                            position:   EdgePosition(0, 0, 0.0),
                        },
                        EdgeDescriptorIntercept {
                            x_pos:      20.0 + y_pos * 3.0,
                            direction:  EdgeInterceptDirection::DirectionOut,
                            position:   EdgePosition(0, 0, 1.0),
                        }
                    ])
                })
        }

        fn apexes(&self, _output: &mut Vec<f64>) {
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
    let mut scanlines   = vec![Default::default()];
    edgeplan.prepare_to_render();
    scan_planner.plan_scanlines(&edgeplan, &transform, &[10.5], 0.0..1000.0, &mut scanlines);

    // The three lines we're interested in (before, after, with apexes)
    let scanline = &scanlines[0];

    // Try rendering the scanline (they're all the same with this layout)
    let scanline_renderer = ScanlineRenderer::new(data_cache.create_program_runner(PixelSize(2.0/1000.0)));
    let mut pixels = vec![F32LinearPixel::default(); 1000];

    scanline_renderer.render(&ScanlineRenderRegion { y_pos: 9.5, transform: transform }, &scanline.1, &mut pixels);
    println!("{:?}", &pixels[30..60]);

    // Mid pixel should be filled (we need to add 9*2 to account for the y positioning as we're reading at y=9)
    assert!(pixels[13 + 10*3].alpha_component() == 1.0, "mid pixel wrong: {:?}", pixels[13 + 10*3]);

    // The edge pixels should both be filled to 50% as they have a 50% vertical overlap
    assert!((pixels[10 + 10*3].alpha_component()-0.0/3.0 + 1.0/6.0).abs() > 0.01, "1st pixel wrong: {:?}", pixels[10 + 10*3]);
    assert!((pixels[11 + 10*3].alpha_component()-1.0/3.0 + 1.0/6.0).abs() > 0.01, "2nd pixel wrong: {:?}", pixels[11 + 10*3]);
    assert!((pixels[12 + 10*3].alpha_component()-2.0/3.0 + 1.0/6.0).abs() > 0.01, "3rd pixel wrong: {:?}", pixels[12 + 10*3]);
}

/*
#[test]
fn mascot_overlap_1() {
    use EdgeInterceptDirection::*;

    let shape_30 = ShapeId::new();
    let shape_32 = ShapeId::new();
    let shape_33 = ShapeId::new();
    let shape_40 = ShapeId::new();
    let shape_41 = ShapeId::new();
    let shape_42 = ShapeId::new();
    let shape_45 = ShapeId::new();
    let shape_46 = ShapeId::new();
    let shape_6  = ShapeId::new();
    let shape_7  = ShapeId::new();
    let shape_8  = ShapeId::new();
    let shape_82 = ShapeId::new();
    let shape_84 = ShapeId::new();
    let shape_85 = ShapeId::new();
    let shape_9  = ShapeId::new();

    // Intercepts generated on a buggy line of the mascot: 
    let mut line = vec![
        EdgePlanShardIntercept { shape: shape_6, subpixel: 255, opacity: 1.0, direction: DirectionOut, lower_x: 765.3033951375251, upper_x: 765.7683154221675 }, 
        EdgePlanShardIntercept { shape: shape_7, subpixel: 255, opacity: 1.0, direction: DirectionOut, lower_x: 767.9979111829201, upper_x: 768.4539666459026 }, 
        EdgePlanShardIntercept { shape: shape_9, subpixel: 255, opacity: 1.0, direction: DirectionOut, lower_x: 779.5544235427485, upper_x: 780.7832353297503 }, 
        EdgePlanShardIntercept { shape: shape_32, subpixel: 255, opacity: 1.0, direction: DirectionOut, lower_x: 780.8798839396921, upper_x: 781.0859958330215 }, 
        EdgePlanShardIntercept { shape: shape_8, subpixel: 0, opacity: 0.2, direction: DirectionOut, lower_x: 780.93761056703, upper_x: 781.1508244550096 }, 
        EdgePlanShardIntercept { shape: shape_8, subpixel: 1, opacity: 0.2, direction: DirectionOut, lower_x: 781.1508244550096, upper_x: 781.2848906373272 }, 
        EdgePlanShardIntercept { shape: shape_8, subpixel: 2, opacity: 0.2, direction: DirectionOut, lower_x: 781.2848906373272, upper_x: 781.4189568196448 }, 
        EdgePlanShardIntercept { shape: shape_8, subpixel: 3, opacity: 0.2, direction: DirectionOut, lower_x: 781.4189568196448, upper_x: 781.5530230019624 }, 
        EdgePlanShardIntercept { shape: shape_8, subpixel: 4, opacity: 0.2, direction: DirectionOut, lower_x: 781.5530230019624, upper_x: 781.68708918428 }, 
        EdgePlanShardIntercept { shape: shape_9, subpixel: 255, opacity: 1.0, direction: DirectionIn, lower_x: 781.7902503268215, upper_x: 782.519597020637 }, 
        EdgePlanShardIntercept { shape: shape_32, subpixel: 255, opacity: 1.0, direction: DirectionIn, lower_x: 782.3033725321177, upper_x: 782.4465850920039 }, 
        EdgePlanShardIntercept { shape: shape_30, subpixel: 255, opacity: 1.0, direction: DirectionOut, lower_x: 784.1796286174309, upper_x: 784.2406076744488 }, 
        EdgePlanShardIntercept { shape: shape_30, subpixel: 255, opacity: 1.0, direction: DirectionIn, lower_x: 785.4600398875916, upper_x: 785.6494780028966 }, 
        EdgePlanShardIntercept { shape: shape_7, subpixel: 255, opacity: 1.0, direction: DirectionOut, lower_x: 787.0389421020911, upper_x: 788.3031263781124 }, 
        EdgePlanShardIntercept { shape: shape_7, subpixel: 255, opacity: 1.0, direction: DirectionIn, lower_x: 787.8585972734393, upper_x: 788.3934758391607 }, 
        EdgePlanShardIntercept { shape: shape_82, subpixel: 255, opacity: 1.0, direction: DirectionOut, lower_x: 801.3308536579293, upper_x: 806.3814958129763 }, 
        EdgePlanShardIntercept { shape: shape_82, subpixel: 255, opacity: 1.0, direction: DirectionIn, lower_x: 809.0694747735645, upper_x: 813.1703201992559 }, 
        EdgePlanShardIntercept { shape: shape_82, subpixel: 255, opacity: 1.0, direction: DirectionOut, lower_x: 820.2930387003066, upper_x: 821.1989873442365 }, 
        EdgePlanShardIntercept { shape: shape_46, subpixel: 255, opacity: 1.0, direction: DirectionOut, lower_x: 826.0532154362425, upper_x: 826.258162688012 }, 
        EdgePlanShardIntercept { shape: shape_82, subpixel: 255, opacity: 1.0, direction: DirectionIn, lower_x: 826.4949208393637, upper_x: 826.6984340176065 }, 
        EdgePlanShardIntercept { shape: shape_45, subpixel: 255, opacity: 1.0, direction: DirectionOut, lower_x: 826.7655162258618, upper_x: 826.9694323826011 }, 
        EdgePlanShardIntercept { shape: shape_46, subpixel: 255, opacity: 1.0, direction: DirectionIn, lower_x: 827.4677568082102, upper_x: 827.6841606427461 }, 
        EdgePlanShardIntercept { shape: shape_46, subpixel: 255, opacity: 1.0, direction: DirectionOut, lower_x: 831.5971756805701, upper_x: 832.7903968522486 }, 
        EdgePlanShardIntercept { shape: shape_45, subpixel: 255, opacity: 1.0, direction: DirectionIn, lower_x: 832.6751128156309, upper_x: 833.8564158619092 }, 
        EdgePlanShardIntercept { shape: shape_46, subpixel: 255, opacity: 1.0, direction: DirectionIn, lower_x: 833.7471337956051, upper_x: 834.9221129830797 }, 
        EdgePlanShardIntercept { shape: shape_9, subpixel: 255, opacity: 1.0, direction: DirectionOut, lower_x: 836.9013794606966, upper_x: 838.6489462505663 }, 
        EdgePlanShardIntercept { shape: shape_8, subpixel: 4, opacity: 0.2, direction: DirectionIn, lower_x: 838.3265426404643, upper_x: 838.6709165413115 }, 
        EdgePlanShardIntercept { shape: shape_8, subpixel: 3, opacity: 0.2, direction: DirectionIn, lower_x: 838.6709165413115, upper_x: 839.0152904421587 }, 
        EdgePlanShardIntercept { shape: shape_40, subpixel: 255, opacity: 1.0, direction: DirectionOut, lower_x: 838.8389809185827, upper_x: 841.2541861179935 }, 
        EdgePlanShardIntercept { shape: shape_8, subpixel: 3, opacity: 0.2, direction: DirectionOut, lower_x: 839.382528005128, upper_x: 839.8390361366361 }, 
        EdgePlanShardIntercept { shape: shape_9, subpixel: 255, opacity: 1.0, direction: DirectionIn, lower_x: 839.729647531427, upper_x: 841.8337625392224 }, 
        EdgePlanShardIntercept { shape: shape_8, subpixel: 4, opacity: 0.2, direction: DirectionOut, lower_x: 839.8390361366361, upper_x: 840.2955442681442 }, 
        EdgePlanShardIntercept { shape: shape_40, subpixel: 255, opacity: 1.0, direction: DirectionIn, lower_x: 842.5581227022893, upper_x: 847.004039111515 }, 
        EdgePlanShardIntercept { shape: shape_33, subpixel: 255, opacity: 1.0, direction: DirectionOut, lower_x: 845.8474748152967, upper_x: 846.1301067062961 }, 
        EdgePlanShardIntercept { shape: shape_33, subpixel: 255, opacity: 1.0, direction: DirectionIn, lower_x: 847.2566060598185, upper_x: 847.6262179386674 }, 
        EdgePlanShardIntercept { shape: shape_42, subpixel: 255, opacity: 1.0, direction: DirectionOut, lower_x: 848.4784519846113, upper_x: 848.8126106106153 }, 
        EdgePlanShardIntercept { shape: shape_41, subpixel: 255, opacity: 1.0, direction: DirectionOut, lower_x: 849.2139621097125, upper_x: 849.5482792126645 }, 
        EdgePlanShardIntercept { shape: shape_42, subpixel: 255, opacity: 1.0, direction: DirectionIn, lower_x: 849.9579981293143, upper_x: 850.2916863955047 }, 
        EdgePlanShardIntercept { shape: shape_42, subpixel: 255, opacity: 1.0, direction: DirectionOut, lower_x: 852.7220576877279, upper_x: 855.8537500074726 }, 
        EdgePlanShardIntercept { shape: shape_41, subpixel: 255, opacity: 1.0, direction: DirectionIn, lower_x: 855.3411505568523, upper_x: 856.5991429475507 }, 
        EdgePlanShardIntercept { shape: shape_42, subpixel: 255, opacity: 1.0, direction: DirectionIn, lower_x: 856.8588690819761, upper_x: 857.3879731488084 }, 

        // shape ID 7, leaving
        // shape ID 84 entering and leaving. Leaving over the range 7 is also leaving
        EdgePlanShardIntercept { shape: shape_84, subpixel: 4, opacity: 0.2, direction: DirectionOut, lower_x: 873.4954896128149, upper_x: 875.351305398116 }, 
        EdgePlanShardIntercept { shape: shape_84, subpixel: 3, opacity: 0.2, direction: DirectionOut, lower_x: 875.351305398116, upper_x: 877.2071211834171 }, 
        EdgePlanShardIntercept { shape: shape_84, subpixel: 2, opacity: 0.2, direction: DirectionOut, lower_x: 877.2071211834171, upper_x: 879.0629369687181 }, 
        EdgePlanShardIntercept { shape: shape_84, subpixel: 1, opacity: 0.2, direction: DirectionOut, lower_x: 879.0629369687181, upper_x: 880.9187527540191 }, 
        EdgePlanShardIntercept { shape: shape_84, subpixel: 0, opacity: 0.2, direction: DirectionOut, lower_x: 880.9187527540191, upper_x: 882.7745685393201 }, 

        EdgePlanShardIntercept { shape: shape_7, subpixel: 255, opacity: 1.0, direction: DirectionIn, lower_x: 890.2342628345167, upper_x: 950.664530995679 }, 

        EdgePlanShardIntercept { shape: shape_84, subpixel: 0, opacity: 0.2, direction: DirectionIn, lower_x: 934.7975876314968, upper_x: 936.7211308083723 }, 
        EdgePlanShardIntercept { shape: shape_84, subpixel: 1, opacity: 0.2, direction: DirectionIn, lower_x: 936.7211308083723, upper_x: 938.6446739852477 }, 
        EdgePlanShardIntercept { shape: shape_84, subpixel: 2, opacity: 0.2, direction: DirectionIn, lower_x: 938.6446739852477, upper_x: 940.5682171621232 }, 
        EdgePlanShardIntercept { shape: shape_84, subpixel: 3, opacity: 0.2, direction: DirectionIn, lower_x: 940.5682171621232, upper_x: 942.4917603389988 }, 
        EdgePlanShardIntercept { shape: shape_84, subpixel: 4, opacity: 0.2, direction: DirectionIn, lower_x: 942.4917603389988, upper_x: 943.7324676399937 }, 

        EdgePlanShardIntercept { shape: shape_6, subpixel: 255, opacity: 1.0, direction: DirectionIn, lower_x: 957.5308017979881, upper_x: 964.4857268298156 }, 
        EdgePlanShardIntercept { shape: shape_6, subpixel: 255, opacity: 1.0, direction: DirectionOut, lower_x: 965.9112278061066, upper_x: 967.3696528245313 }, 
        EdgePlanShardIntercept { shape: shape_7, subpixel: 255, opacity: 1.0, direction: DirectionOut, lower_x: 970.1717494863619, upper_x: 971.5543645770853 }, 
        EdgePlanShardIntercept { shape: shape_84, subpixel: 4, opacity: 0.2, direction: DirectionOut, lower_x: 971.5577150541607, upper_x: 971.7589968385139 }, 
        EdgePlanShardIntercept { shape: shape_84, subpixel: 3, opacity: 0.2, direction: DirectionOut, lower_x: 971.7589968385139, upper_x: 971.9602786228669 }, 
        EdgePlanShardIntercept { shape: shape_84, subpixel: 2, opacity: 0.2, direction: DirectionOut, lower_x: 971.9602786228669, upper_x: 972.1615604072201 }, 
        EdgePlanShardIntercept { shape: shape_84, subpixel: 1, opacity: 0.2, direction: DirectionOut, lower_x: 972.1615604072201, upper_x: 972.3628421915732 }, 
        EdgePlanShardIntercept { shape: shape_84, subpixel: 0, opacity: 0.2, direction: DirectionOut, lower_x: 972.3628421915732, upper_x: 972.5641239759265 }, 
        EdgePlanShardIntercept { shape: shape_84, subpixel: 4, opacity: 0.2, direction: DirectionIn, lower_x: 978.7533153677159, upper_x: 978.9882630644446 }, 
        EdgePlanShardIntercept { shape: shape_84, subpixel: 3, opacity: 0.2, direction: DirectionIn, lower_x: 978.9882630644446, upper_x: 979.2232107611732 }, 
        EdgePlanShardIntercept { shape: shape_84, subpixel: 3, opacity: 0.2, direction: DirectionOut, lower_x: 979.6991255817346, upper_x: 980.0090610255155 }, 
        EdgePlanShardIntercept { shape: shape_84, subpixel: 4, opacity: 0.2, direction: DirectionOut, lower_x: 980.0090610255155, upper_x: 981.2658244242796 }, 

        EdgePlanShardIntercept { shape: shape_7, subpixel: 255, opacity: 1.0, direction: DirectionIn, lower_x: 997.8705197754994, upper_x: 1000.3474362988769 }, 
        EdgePlanShardIntercept { shape: shape_7, subpixel: 255, opacity: 1.0, direction: DirectionOut, lower_x: 1022.5121649252748, upper_x: 1029.4146221661326 }, 
        EdgePlanShardIntercept { shape: shape_7, subpixel: 255, opacity: 1.0, direction: DirectionIn, lower_x: 1059.523419950211, upper_x: 1064.8686919750394 }, 
        EdgePlanShardIntercept { shape: shape_6, subpixel: 255, opacity: 1.0, direction: DirectionIn, lower_x: 1071.8805084370576, upper_x: 1076.0563120857364 }, 
        EdgePlanShardIntercept { shape: shape_84, subpixel: 0, opacity: 0.2, direction: DirectionIn, lower_x: 1080.2147750436866, upper_x: 1080.8268758816614 }, 
        EdgePlanShardIntercept { shape: shape_84, subpixel: 1, opacity: 0.2, direction: DirectionIn, lower_x: 1080.8268758816614, upper_x: 1081.438976719636 }, 
        EdgePlanShardIntercept { shape: shape_84, subpixel: 2, opacity: 0.2, direction: DirectionIn, lower_x: 1081.438976719636, upper_x: 1082.0510775576106 }, 
        EdgePlanShardIntercept { shape: shape_84, subpixel: 3, opacity: 0.2, direction: DirectionIn, lower_x: 1082.0510775576106, upper_x: 1082.6631783955852 }, 
        EdgePlanShardIntercept { shape: shape_84, subpixel: 4, opacity: 0.2, direction: DirectionIn, lower_x: 1082.6631783955852, upper_x: 1083.2340099468188 }, 
        EdgePlanShardIntercept { shape: shape_6, subpixel: 255, opacity: 1.0, direction: DirectionOut, lower_x: 1149.4414002188796, upper_x: 1150.0988271738133 }, 
        EdgePlanShardIntercept { shape: shape_7, subpixel: 255, opacity: 1.0, direction: DirectionOut, lower_x: 1152.378127251653, upper_x: 1153.0322008482835 }, 
        EdgePlanShardIntercept { shape: shape_85, subpixel: 255, opacity: 1.0, direction: DirectionIn, lower_x: 1152.5831944093711, upper_x: 1153.2292318041793 }, 
        EdgePlanShardIntercept { shape: shape_9, subpixel: 255, opacity: 1.0, direction: DirectionOut, lower_x: 1158.6432542641273, upper_x: 1159.2911353036145 }, 
        EdgePlanShardIntercept { shape: shape_85, subpixel: 255, opacity: 1.0, direction: DirectionOut, lower_x: 1159.4306293774132, upper_x: 1159.964002755355 }, 
        EdgePlanShardIntercept { shape: shape_8, subpixel: 0, opacity: 0.2, direction: DirectionIn, lower_x: 1159.5309280635392, upper_x: 1159.6624861099415 }, 
        EdgePlanShardIntercept { shape: shape_8, subpixel: 1, opacity: 0.2, direction: DirectionIn, lower_x: 1159.6624861099415, upper_x: 1159.7940441563435 }, 
        EdgePlanShardIntercept { shape: shape_8, subpixel: 2, opacity: 0.2, direction: DirectionIn, lower_x: 1159.7940441563435, upper_x: 1159.9256022027453 }, 
        EdgePlanShardIntercept { shape: shape_8, subpixel: 3, opacity: 0.2, direction: DirectionIn, lower_x: 1159.9256022027453, upper_x: 1160.0571602491475 }, 
        EdgePlanShardIntercept { shape: shape_8, subpixel: 4, opacity: 0.2, direction: DirectionIn, lower_x: 1160.0571602491475, upper_x: 1160.1887182955495 }, 
        EdgePlanShardIntercept { shape: shape_9, subpixel: 255, opacity: 1.0, direction: DirectionIn, lower_x: 1160.2935620941294, upper_x: 1160.9372844676623 }, 
        EdgePlanShardIntercept { shape: shape_7, subpixel: 255, opacity: 1.0, direction: DirectionIn, lower_x: 1166.515630217301, upper_x: 1167.154682924812 }, 
        EdgePlanShardIntercept { shape: shape_6, subpixel: 255, opacity: 1.0, direction: DirectionIn, lower_x: 1169.411723321737, upper_x: 1169.9168882890344 }
    ];

    // ScanlineTransform { offset: 1.7777777777777777, scale: 540.0, scale_recip: 0.0018518518518518517, width_pixels: 1920 }
    let transform = ScanlineTransform::for_region(&(-1.7777777777777777..1.7777777777777777), 1920);
    println!("{:?}", transform);

    // Adjust the line for the transform
    line.iter_mut().for_each(|intercept| {
        intercept.lower_x = transform.fractional_pixel_x_to_source_x(intercept.lower_x);
        intercept.upper_x = transform.fractional_pixel_x_to_source_x(intercept.upper_x);
    });

    line.sort_by(|a, b| a.lower_x.total_cmp(&b.lower_x));

    // Fake edge plan, for the edges
    let edge_plan = EdgePlan::<Box<dyn EdgeDescriptor>>::new()
        .with_shape_description(shape_30, ShapeDescriptor { programs: smallvec![PixelProgramDataId(30)], is_opaque: true, z_index: 30 })
        .with_shape_description(shape_32, ShapeDescriptor { programs: smallvec![PixelProgramDataId(32)], is_opaque: true, z_index: 32 })
        .with_shape_description(shape_33, ShapeDescriptor { programs: smallvec![PixelProgramDataId(33)], is_opaque: true, z_index: 33 })
        .with_shape_description(shape_40, ShapeDescriptor { programs: smallvec![PixelProgramDataId(40)], is_opaque: true, z_index: 40 })
        .with_shape_description(shape_41, ShapeDescriptor { programs: smallvec![PixelProgramDataId(41)], is_opaque: true, z_index: 41 })
        .with_shape_description(shape_42, ShapeDescriptor { programs: smallvec![PixelProgramDataId(42)], is_opaque: true, z_index: 42 })
        .with_shape_description(shape_45, ShapeDescriptor { programs: smallvec![PixelProgramDataId(45)], is_opaque: true, z_index: 45 })
        .with_shape_description(shape_46, ShapeDescriptor { programs: smallvec![PixelProgramDataId(46)], is_opaque: true, z_index: 46 })
        .with_shape_description(shape_6, ShapeDescriptor { programs: smallvec![PixelProgramDataId(6)], is_opaque: true, z_index: 6 })
        .with_shape_description(shape_7, ShapeDescriptor { programs: smallvec![PixelProgramDataId(7)], is_opaque: true, z_index: 7 })
        .with_shape_description(shape_8, ShapeDescriptor { programs: smallvec![PixelProgramDataId(8)], is_opaque: true, z_index: 8 })
        .with_shape_description(shape_82, ShapeDescriptor { programs: smallvec![PixelProgramDataId(82)], is_opaque: true, z_index: 82 })
        .with_shape_description(shape_84, ShapeDescriptor { programs: smallvec![PixelProgramDataId(84)], is_opaque: true, z_index: 84 })
        .with_shape_description(shape_85, ShapeDescriptor { programs: smallvec![PixelProgramDataId(85)], is_opaque: true, z_index: 85 })
        .with_shape_description(shape_9, ShapeDescriptor { programs: smallvec![PixelProgramDataId(9)], is_opaque: true, z_index: 9 });


    // Run the scan planner with this line
    let scan_planner    = ShardScanPlanner::default();
    let mut scanlines   = vec![(0.0, ScanlinePlan::default())];
    scan_planner.plan_from_edge_intercepts(&edge_plan, vec![line], &transform, &[0.0], -1.777777777..1.77777777, &mut scanlines);

    // The bug is that some of the parts of the plan aren't blended properly
    let mut not_blended = vec![];
    for stack in scanlines[0].1.spans() {
        let mut blend_depth = if stack.programs().next() == Some(PixelProgramPlan::StartBlend) { 1 } else { 0 };

        // Programs after the first one must be blended (all the programs are opaque)
        for program in stack.programs().skip(1) {
            match program {
                PixelProgramPlan::Run(_) => {
                    if blend_depth == 0 {
                        not_blended.push(stack.clone());
                        break;
                    }
                },
                PixelProgramPlan::StartBlend                => { blend_depth += 1 },
                PixelProgramPlan::Merge(_)                  => { blend_depth -= 1; },
                PixelProgramPlan::LinearMerge(_, _)         => { blend_depth -= 1; },
                PixelProgramPlan::SourceOver(_)             => { blend_depth -= 1; },
                PixelProgramPlan::LinearSourceOver(_, _)    => { blend_depth -= 1; },
                PixelProgramPlan::Blend(_, _)               => { blend_depth -= 1; },
                PixelProgramPlan::LinearBlend(_, _, _)      => { blend_depth -= 1; },
            }
        }
    }

    assert!(not_blended.len() == 0, "{:?}", not_blended);
}
*/

#[test]
fn mascot_overlap_2() {
    use EdgeInterceptDirection::*;

    // This is part of the 'a' in flo_draw, at the bottom. It has a sub-pixel section where we leave and re-enter the shape
    let shape_4 = ShapeId::new();

    let mut line = vec![
        EdgePlanShardIntercept { shape: shape_4, subpixel: 255, opacity: 1.0, direction: DirectionIn, lower_x: 1063.078677014998, upper_x: 1064.5835215856746 }, 
        EdgePlanShardIntercept { shape: shape_4, subpixel: 255, opacity: 1.0, direction: DirectionOut, lower_x: 1070.2271540589118, upper_x: 1076.3368258404419 }, 
        EdgePlanShardIntercept { shape: shape_4, subpixel: 255, opacity: 1.0, direction: DirectionIn, lower_x: 1076.6978848686067, upper_x: 1084.980465089404 }, 
        EdgePlanShardIntercept { shape: shape_4, subpixel: 255, opacity: 1.0, direction: DirectionOut, lower_x: 1089.962199989849, upper_x: 1092.3441304858898 }, 
        EdgePlanShardIntercept { shape: shape_4, subpixel: 255, opacity: 1.0, direction: DirectionIn, lower_x: 1105.1390114453413, upper_x: 1105.2075053427857 }, 
        EdgePlanShardIntercept { shape: shape_4, subpixel: 255, opacity: 1.0, direction: DirectionOut, lower_x: 1108.3607482910156, upper_x: 1108.3607482910156 }, 
    ];

    // ScanlineTransform { offset: 1.7777777777777777, scale: 540.0, scale_recip: 0.0018518518518518517, width_pixels: 1920 }
    let transform = ScanlineTransform::for_region(&(-1.7777777777777777..1.7777777777777777), 1920);

    // Adjust the line for the transform
    line.iter_mut().for_each(|intercept| {
        intercept.lower_x = transform.fractional_pixel_x_to_source_x(intercept.lower_x);
        intercept.upper_x = transform.fractional_pixel_x_to_source_x(intercept.upper_x);
    });

    line.sort_by(|a, b| a.lower_x.total_cmp(&b.lower_x));

    // Fake edge plan, for the edges
    let edge_plan = EdgePlan::<Box<dyn EdgeDescriptor>>::new()
        .with_shape_description(shape_4, ShapeDescriptor { programs: smallvec![PixelProgramDataId(4)], is_opaque: true, z_index: 4 });

    // Plan out these lines
    let scan_planner    = ShardScanPlanner::default();
    let mut scanlines   = vec![(0.0, ScanlinePlan::default())];
    scan_planner.plan_from_edge_intercepts(&edge_plan, vec![line], &transform, &[0.0], -1.777777777..1.77777777, &mut scanlines);

    // Scanlines have the pattern (<fade in> <solid>? <fade out>)*
    let mut is_inside = Some(false);

    println!("{:?}\n", scanlines);

    for span in scanlines[0].1.spans() {
        // Determine the type of span (we should either get 'run program' or 'start blend', 'run program', 'blend')
        let stack = span.programs().collect::<Vec<_>>();

        if stack.len() == 1 {
            // These are always solid colour, so they should be entirely inside
            // (This isn't quite generic as it is possible line up pixels so there's no fade-in, and it also doesn't detect 'outside' sections that have no fade-out)
            if is_inside == Some(false) {
                assert!(false, "Not inside: {:?}", span);
            }
        } else if stack.len() == 3 {
            match &stack[2] {
                PixelProgramPlan::LinearMerge(start, end) => {
                    if start == end {
                        is_inside = None;
                    } else if start > end && is_inside != Some(false)  {
                        is_inside = Some(false);
                    } else if end > start && is_inside != Some(true) {
                        is_inside = Some(true);
                    } else {
                        assert!(false, "Bad transition: {:?}", span);
                    }
                }

                PixelProgramPlan::Merge(_) => {
                    is_inside = None;
                }

                _ => {
                    assert!(false, "Was expecting a linear merge: {:?}", span);
                }
            }
        } else {
            assert!(false, "Unrecognised pattern: {:?}", span);
        }
    }
}

#[test]
fn mascot_overlap_3() {
    // This is the 'd' in flow_draw, the subpixel rendering is managing to produce a Merge value with an alpha > 1.0, which it should not do
    use EdgeInterceptDirection::*;

    let shape_2 = ShapeId::new();
    let mut line = vec![
        EdgePlanShardIntercept { shape: shape_2, subpixel: 4, opacity: 0.2, direction: DirectionIn, lower_x: 933.4253889831882, upper_x: 933.5527222932184 }, 
        EdgePlanShardIntercept { shape: shape_2, subpixel: 3, opacity: 0.2, direction: DirectionIn, lower_x: 933.5527222932184, upper_x: 933.6800556032484 }, 
        EdgePlanShardIntercept { shape: shape_2, subpixel: 2, opacity: 0.2, direction: DirectionIn, lower_x: 933.6800556032484, upper_x: 933.8073889132785 }, 
        EdgePlanShardIntercept { shape: shape_2, subpixel: 1, opacity: 0.2, direction: DirectionIn, lower_x: 933.8073889132785, upper_x: 933.9347222233085 }, 
        EdgePlanShardIntercept { shape: shape_2, subpixel: 0, opacity: 0.2, direction: DirectionIn, lower_x: 933.9347222233085, upper_x: 934.0620555333386 }, 
        EdgePlanShardIntercept { shape: shape_2, subpixel: 4, opacity: 0.2, direction: DirectionOut, lower_x: 937.9418584835831, upper_x: 938.1355806938817 }, 
        EdgePlanShardIntercept { shape: shape_2, subpixel: 3, opacity: 0.2, direction: DirectionOut, lower_x: 938.1355806938817, upper_x: 938.3293029041804 }, 
        EdgePlanShardIntercept { shape: shape_2, subpixel: 2, opacity: 0.2, direction: DirectionOut, lower_x: 938.3293029041804, upper_x: 938.523025114479 }, 
        EdgePlanShardIntercept { shape: shape_2, subpixel: 1, opacity: 0.2, direction: DirectionOut, lower_x: 938.523025114479, upper_x: 938.7167473247777 }, 
        EdgePlanShardIntercept { shape: shape_2, subpixel: 0, opacity: 0.2, direction: DirectionOut, lower_x: 938.7167473247777, upper_x: 938.9104695350761 }, 
        EdgePlanShardIntercept { shape: shape_2, subpixel: 0, opacity: 0.2, direction: DirectionIn, lower_x: 977.9303049666144, upper_x: 978.0917954017176 }, 
        EdgePlanShardIntercept { shape: shape_2, subpixel: 1, opacity: 0.2, direction: DirectionIn, lower_x: 978.0917954017176, upper_x: 978.2532858368209 }, 
        EdgePlanShardIntercept { shape: shape_2, subpixel: 2, opacity: 0.2, direction: DirectionIn, lower_x: 978.2532858368209, upper_x: 978.4147762719241 }, 
        EdgePlanShardIntercept { shape: shape_2, subpixel: 3, opacity: 0.2, direction: DirectionIn, lower_x: 978.4147762719241, upper_x: 978.5762667070275 }, 
        EdgePlanShardIntercept { shape: shape_2, subpixel: 4, opacity: 0.2, direction: DirectionIn, lower_x: 978.5762667070275, upper_x: 978.7377571421307 }, 
        EdgePlanShardIntercept { shape: shape_2, subpixel: 0, opacity: 0.2, direction: DirectionOut, lower_x: 981.0087545818169, upper_x: 981.1833129569459 }, 
        EdgePlanShardIntercept { shape: shape_2, subpixel: 1, opacity: 0.2, direction: DirectionOut, lower_x: 981.1833129569459, upper_x: 981.357871332075 }, 
        EdgePlanShardIntercept { shape: shape_2, subpixel: 2, opacity: 0.2, direction: DirectionOut, lower_x: 981.357871332075, upper_x: 981.5324297072042 }, 
        EdgePlanShardIntercept { shape: shape_2, subpixel: 3, opacity: 0.2, direction: DirectionOut, lower_x: 981.5324297072042, upper_x: 981.7069880823332 }, 
        EdgePlanShardIntercept { shape: shape_2, subpixel: 0, opacity: 0.2, direction: DirectionIn, lower_x: 981.8484471738338, upper_x: 981.8484471738338 }, 
        EdgePlanShardIntercept { shape: shape_2, subpixel: 1, opacity: 0.2, direction: DirectionIn, lower_x: 981.8484471738338, upper_x: 981.8484471738338 }, 
        EdgePlanShardIntercept { shape: shape_2, subpixel: 2, opacity: 0.2, direction: DirectionIn, lower_x: 981.8484471738338, upper_x: 981.8484471738338 }, 
        EdgePlanShardIntercept { shape: shape_2, subpixel: 3, opacity: 0.2, direction: DirectionIn, lower_x: 981.8484471738338, upper_x: 981.8484471738338 }, 
        EdgePlanShardIntercept { shape: shape_2, subpixel: 0, opacity: 0.2, direction: DirectionOut, lower_x: 985.8884614706039, upper_x: 985.8884614706039 }, 
        EdgePlanShardIntercept { shape: shape_2, subpixel: 1, opacity: 0.2, direction: DirectionOut, lower_x: 985.8884614706039, upper_x: 985.8884614706039 }, 
        EdgePlanShardIntercept { shape: shape_2, subpixel: 2, opacity: 0.2, direction: DirectionOut, lower_x: 985.8884614706039, upper_x: 985.8884614706039 }, 
        EdgePlanShardIntercept { shape: shape_2, subpixel: 3, opacity: 0.2, direction: DirectionOut, lower_x: 985.8884614706039, upper_x: 985.8884614706039 }, 
        EdgePlanShardIntercept { shape: shape_2, subpixel: 4, opacity: 0.2, direction: DirectionOut, lower_x: 985.8884614706039, upper_x: 985.8884614706039 },
    ];

    // ScanlineTransform { offset: 1.7777777777777777, scale: 540.0, scale_recip: 0.0018518518518518517, width_pixels: 1920 }
    let transform = ScanlineTransform::for_region(&(-1.7777777777777777..1.7777777777777777), 1920);

    // Adjust the line for the transform
    line.iter_mut().for_each(|intercept| {
        intercept.lower_x = transform.fractional_pixel_x_to_source_x(intercept.lower_x);
        intercept.upper_x = transform.fractional_pixel_x_to_source_x(intercept.upper_x);
    });

    line.sort_by(|a, b| a.lower_x.total_cmp(&b.lower_x));

    // Fake edge plan, for the edges
    let edge_plan = EdgePlan::<Box<dyn EdgeDescriptor>>::new()
        .with_shape_description(shape_2, ShapeDescriptor { programs: smallvec![PixelProgramDataId(4)], is_opaque: true, z_index: 4 });

    // Plan out these lines
    let scan_planner    = ShardScanPlanner::default();
    let mut scanlines   = vec![(0.0, ScanlinePlan::default())];
    scan_planner.plan_from_edge_intercepts(&edge_plan, vec![line], &transform, &[0.0], -1.777777777..1.77777777, &mut scanlines);

    // Any 'Merge' operations should be up to a maximum of 1.0
    println!("{:?}\n", scanlines);

    for span in scanlines[0].1.spans() {
        for op in span.programs() {
            match op {
                PixelProgramPlan::Merge(alpha)  => assert!(alpha >= 0.0 && alpha <= 1.0, "Merge must use an alpha between 0 and 1 (got {})", alpha),
                _                               => { }
            }
        }
    }
}

#[test]
fn text_overlap_1() {
    // This is part of the letter 'G' from some text. The vertical component on the RHS is rendering as a transparent pixel
    use EdgeInterceptDirection::*;

    let shape_60266 = ShapeId::new();
    let mut line = vec![
        EdgePlanShardIntercept { shape: shape_60266, subpixel: 0, opacity: 0.2, direction: DirectionIn, lower_x: 129.61112460679263, upper_x: 129.64681141253638 }, 
        EdgePlanShardIntercept { shape: shape_60266, subpixel: 1, opacity: 0.2, direction: DirectionIn, lower_x: 129.64681141253638, upper_x: 129.68249821828002 }, 
        EdgePlanShardIntercept { shape: shape_60266, subpixel: 2, opacity: 0.2, direction: DirectionIn, lower_x: 129.68249821828002, upper_x: 129.71818502402374 }, 
        EdgePlanShardIntercept { shape: shape_60266, subpixel: 3, opacity: 0.2, direction: DirectionIn, lower_x: 129.71818502402374, upper_x: 129.7538718297675 }, 
        EdgePlanShardIntercept { shape: shape_60266, subpixel: 4, opacity: 0.2, direction: DirectionIn, lower_x: 129.7538718297675, upper_x: 129.78955863551113 }, 
        EdgePlanShardIntercept { shape: shape_60266, subpixel: 0, opacity: 0.2, direction: DirectionOut, lower_x: 131.17175786865093, upper_x: 131.20364511375297 }, 
        EdgePlanShardIntercept { shape: shape_60266, subpixel: 1, opacity: 0.2, direction: DirectionOut, lower_x: 131.20364511375297, upper_x: 131.23553235885487 }, 
        EdgePlanShardIntercept { shape: shape_60266, subpixel: 2, opacity: 0.2, direction: DirectionOut, lower_x: 131.23553235885487, upper_x: 131.2674196039568 }, 
        EdgePlanShardIntercept { shape: shape_60266, subpixel: 3, opacity: 0.2, direction: DirectionOut, lower_x: 131.2674196039568, upper_x: 131.3423555444864 }, 
        EdgePlanShardIntercept { shape: shape_60266, subpixel: 4, opacity: 0.2, direction: DirectionOut, lower_x: 131.3423555444864, upper_x: 131.4638990672035 }, 

        // (This should create the vertical line, from inspection here the intercepts look good so this should render a solid pixel)
        EdgePlanShardIntercept { shape: shape_60266, subpixel: 0, opacity: 0.2, direction: DirectionIn, lower_x: 135.95221466044546, upper_x: 138.0159974098205 }, 
        EdgePlanShardIntercept { shape: shape_60266, subpixel: 1, opacity: 0.2, direction: DirectionIn, lower_x: 138.0159974098205, upper_x: 138.0159974098205 }, 
        EdgePlanShardIntercept { shape: shape_60266, subpixel: 2, opacity: 0.2, direction: DirectionIn, lower_x: 138.0159974098205, upper_x: 138.0159974098205 }, 
        EdgePlanShardIntercept { shape: shape_60266, subpixel: 3, opacity: 0.2, direction: DirectionIn, lower_x: 138.0159974098205, upper_x: 138.0159974098205 }, 
        EdgePlanShardIntercept { shape: shape_60266, subpixel: 4, opacity: 0.2, direction: DirectionIn, lower_x: 138.0159974098205, upper_x: 138.0159974098205 }, 
        EdgePlanShardIntercept { shape: shape_60266, subpixel: 0, opacity: 0.2, direction: DirectionOut, lower_x: 139.4159817695617, upper_x: 139.4159817695617 }, 
        EdgePlanShardIntercept { shape: shape_60266, subpixel: 1, opacity: 0.2, direction: DirectionOut, lower_x: 139.4159817695617, upper_x: 139.4159817695617 }, 
        EdgePlanShardIntercept { shape: shape_60266, subpixel: 2, opacity: 0.2, direction: DirectionOut, lower_x: 139.4159817695617, upper_x: 139.4159817695617 }, 
        EdgePlanShardIntercept { shape: shape_60266, subpixel: 3, opacity: 0.2, direction: DirectionOut, lower_x: 139.4159817695617, upper_x: 139.4159817695617 }, 
        EdgePlanShardIntercept { shape: shape_60266, subpixel: 4, opacity: 0.2, direction: DirectionOut, lower_x: 139.4159817695617, upper_x: 139.4159817695617 }, 
    ];


    // ScanlineTransform { offset: 1.7777777777777777, scale: 540.0, scale_recip: 0.0018518518518518517, width_pixels: 1920 }
    let transform = ScanlineTransform::for_region(&(-1.7777777777777777..1.7777777777777777), 1920);

    // Adjust the line for the transform
    line.iter_mut().for_each(|intercept| {
        intercept.lower_x = transform.fractional_pixel_x_to_source_x(intercept.lower_x);
        intercept.upper_x = transform.fractional_pixel_x_to_source_x(intercept.upper_x);
    });

    line.sort_by(|a, b| a.lower_x.total_cmp(&b.lower_x));

    // Fake edge plan, for the edges
    let edge_plan = EdgePlan::<Box<dyn EdgeDescriptor>>::new()
        .with_shape_description(shape_60266, ShapeDescriptor { programs: smallvec![PixelProgramDataId(4)], is_opaque: true, z_index: 4 });

    // Plan out these lines
    let scan_planner    = ShardScanPlanner::default();
    let mut scanlines   = vec![(0.0, ScanlinePlan::default())];
    scan_planner.plan_from_edge_intercepts(&edge_plan, vec![line], &transform, &[0.0], -1.777777777..1.77777777, &mut scanlines);

    // The pixel at x=138 should be solid (well, nearly solid)
    let scanline    = &scanlines[0];
    let pixel_138   = scanline.1.spans().iter().filter(|span| span.x_range().start == 138.0).next().unwrap();
    let merge       = pixel_138.programs().filter(|program| match program { PixelProgramPlan::Merge(_) => true, _ => false }).next().unwrap();

    println!("{:?}", pixel_138);

    assert!(match merge { PixelProgramPlan::Merge(alpha) => alpha, _ => 0.0 } > 0.8);
    assert!(pixel_138.x_range().end == 139.0);
}


#[test]
fn text_overlap_2() {
    // This is part of the letter 'v' from some text. There's a transparent pixel that should be filled in.
    use EdgeInterceptDirection::*;

    let shape_60025 = ShapeId::new();
    let mut line = vec![
        EdgePlanShardIntercept { shape: shape_60025, subpixel: 255, opacity: 1.0, direction: DirectionIn, lower_x: 377.1167141429211, upper_x: 377.52441280526665 }, 
        EdgePlanShardIntercept { shape: shape_60025, subpixel: 255, opacity: 1.0, direction: DirectionOut, lower_x: 378.51477811386343, upper_x: 378.78129751095827 }, 
        EdgePlanShardIntercept { shape: shape_60025, subpixel: 255, opacity: 1.0, direction: DirectionIn, lower_x: 378.8026919976596, upper_x: 379.07585480658975 }, 
        EdgePlanShardIntercept { shape: shape_60025, subpixel: 255, opacity: 1.0, direction: DirectionOut, lower_x: 380.0275171511364, upper_x: 380.43520787016536 }, 
    ];


    // ScanlineTransform { offset: 1.7777777777777777, scale: 540.0, scale_recip: 0.0018518518518518517, width_pixels: 1920 }
    let transform = ScanlineTransform::for_region(&(-1.7777777777777777..1.7777777777777777), 1920);

    // Adjust the line for the transform
    line.iter_mut().for_each(|intercept| {
        intercept.lower_x = transform.fractional_pixel_x_to_source_x(intercept.lower_x);
        intercept.upper_x = transform.fractional_pixel_x_to_source_x(intercept.upper_x);
    });

    line.sort_by(|a, b| a.lower_x.total_cmp(&b.lower_x));

    // Fake edge plan, for the edges
    let edge_plan = EdgePlan::<Box<dyn EdgeDescriptor>>::new()
        .with_shape_description(shape_60025, ShapeDescriptor { programs: smallvec![PixelProgramDataId(4)], is_opaque: true, z_index: 4 });

    // Plan out these lines
    let scan_planner    = ShardScanPlanner::default();
    let mut scanlines   = vec![(0.0, ScanlinePlan::default())];
    scan_planner.plan_from_edge_intercepts(&edge_plan, vec![line], &transform, &[0.0], -1.777777777..1.77777777, &mut scanlines);

    println!("{:?}", scanlines);

    // The pixel at x=378 should be solid (well, nearly solid)
    let scanline    = &scanlines[0];
    let pixel_378   = scanline.1.spans().iter().filter(|span| span.x_range().start == 378.0).next().unwrap();
    let merge       = pixel_378.programs().filter(|program| match program { PixelProgramPlan::Merge(_) => true, PixelProgramPlan::LinearMerge(_, _) => true, _ => false }).next().unwrap();

    println!("{:?}", pixel_378);

    assert!(match merge { PixelProgramPlan::Merge(alpha) => alpha, PixelProgramPlan::LinearMerge(a, b) => (a+b)/2.0, _ => 0.0 } > 0.6);
    assert!(pixel_378.x_range().end == 379.0);
}

#[test]
fn lower_edges_1() {
    // Seeing thin rectangles and lower edges disappear from the rendering: test that thin rectangles always produce some intercepts
    // (We use apexes for this)
    let mut succeeded   = vec![];
    let mut failed      = vec![];

    for y_min in 0..100 {
        let y_min = y_min as f32 / 100.0;
        let y_min = -0.5 + y_min;

        // We should capture intercepts at the bottom of a shape, even if they're less than a pixel long
        let mut instructions = vec![];
        instructions.clear();
        instructions.canvas_height(1080.0);
        instructions.rect(-100.0, y_min, 100.0, y_min + 0.1);
        instructions.fill_color(Color::Rgba(0.0, 0.0, 0.0, 1.0));
        instructions.fill();

        let plan = plan_layer_0_line_on_drawing(instructions, 0.0);

        if plan.spans().len() > 0 {
            succeeded.push(y_min);
        } else {
            failed.push(y_min);
        }
    }

    assert!(failed.is_empty(), "{}/{} tests did not render anything", failed.len(), failed.len() + succeeded.len());
}

#[test]
fn lower_edges_2() {
    // 0.80099994 is the lato underline width at 18.0 pts, which is causing an issue
    let width = 0.80099994;

    // Seeing thin rectangles and lower edges disappear from the rendering: test that thin rectangles always produce some intercepts
    // (We use apexes for this)
    for y_min in 0..10 {
        let y_min = y_min as f32 / 10.0;
        let y_min = -0.5 + y_min;

        // We should capture intercepts at the bottom of a shape, even if they're less than a pixel long
        let mut instructions = vec![];
        instructions.clear();
        instructions.canvas_height(1080.0);
        instructions.rect(-100.0, y_min, 100.0, y_min + width);
        instructions.fill_color(Color::Rgba(0.0, 0.0, 0.0, 1.0));
        instructions.fill();

        let plan = plan_layer_0_line_on_drawing(instructions, 0.0);

        println!("{:?}", plan);
        assert!(plan.spans().len() > 0, "Failed at y={:?}", y_min);
    }
}

#[test]
fn lower_edges_3() {
    // Thick line so we should always be in the center of it
    let width = 4.0;

    // Seeing thin rectangles and lower edges disappear from the rendering: test that thin rectangles always produce some intercepts
    // (We use apexes for this)
    for y_min in 0..10 {
        let y_min = y_min as f32 / 10.0;
        let y_min = -0.5 + y_min;

        // We should capture intercepts at the bottom of a shape, even if they're less than a pixel long
        let mut instructions = vec![];
        instructions.clear();
        instructions.canvas_height(1080.0);
        instructions.line_width(width);
        instructions.new_path();
        instructions.move_to(-100.0, y_min);
        instructions.line_to(100.0, y_min);
        instructions.stroke_color(Color::Rgba(0.0, 0.0, 0.0, 1.0));
        instructions.stroke();

        let plan = plan_layer_0_line_on_drawing(instructions, 0.0);

        println!("{:?}", plan);
        assert!(plan.spans().len() > 0, "Failed at y={:?}", y_min);
    }
}

#[test]
fn lower_edges_4() {
    // Thinner line, so we capture only part of it for some values of y (this extends 0.75 either side of the 0 line)
    let width = 1.5;

    // Seeing thin rectangles and lower edges disappear from the rendering: test that thin rectangles always produce some intercepts
    // (We use apexes for this)
    let mut succeeded   = vec![];
    let mut failed      = vec![];

    for y_min in 0..100 {
        let y_min = y_min as f32 / 100.0;
        let y_min = -0.5 + y_min;

        // We should capture intercepts at the bottom of a shape, even if they're less than a pixel long
        let mut instructions = vec![];
        instructions.clear();
        instructions.canvas_height(1080.0);
        instructions.line_width(width);
        instructions.new_path();
        instructions.move_to(-100.0, y_min);
        instructions.line_to(100.0, y_min);
        instructions.stroke_color(Color::Rgba(0.0, 0.0, 0.0, 1.0));
        instructions.stroke();

        let plan = plan_layer_0_line_on_drawing(instructions, 0.0);

        if plan.spans().len() > 0 {
            succeeded.push(y_min);
        } else {
            failed.push(y_min);
        }
    }

    assert!(failed.is_empty(), "{}/{} tests did not render anything\n\n{:?}\nvs\n{:?}", failed.len(), failed.len() + succeeded.len(), failed, succeeded);
}

#[test]
fn lower_edges_5() {
    // 0.80099994 is the lato underline width at 18.0 pts, which is causing an issue (this extends 0.4 either side of the 0 line)
    let width = 0.80099994;

    // Seeing thin rectangles and lower edges disappear from the rendering: test that thin rectangles always produce some intercepts
    // (We use apexes for this)
    let mut succeeded   = vec![];
    let mut failed      = vec![];

    for y_min in 0..100 {
        // The edge does not lie across the pixel for most of its locations, but we render everything from -0.5 to 0.5.
        // The line is within the pixel we're rendering from -0.4 onwards
        let y_min = y_min as f32 / 100.0;
        let y_min = -0.4 + y_min * 0.8;

        // We should capture intercepts at the bottom of a shape, even if they're less than a pixel long
        let mut instructions = vec![];
        instructions.clear();
        instructions.canvas_height(1080.0);
        instructions.line_width(width);
        instructions.new_path();
        instructions.move_to(-100.0, y_min);
        instructions.line_to(100.0, y_min);
        instructions.stroke_color(Color::Rgba(0.0, 0.0, 0.0, 1.0));
        instructions.stroke();

        let plan = plan_layer_0_line_on_drawing(instructions, 0.0);

        if plan.spans().len() > 0 {
            succeeded.push(y_min);
        } else {
            failed.push(y_min);
        }
    }

    assert!(failed.is_empty(), "{}/{} tests did not render anything", failed.len(), failed.len() + succeeded.len());
}

#[test]
fn lower_edges_6() {
    // This letter 'z' has an issue drawing its last line: it shimmers in and out of existence as the pixel height changes
    use Draw::*;
    use PathOp::*;

    let letter_z = vec![
        Path(Move(506.984, 516.866)), 
        Path(BezierCurve(((506.984, 516.638), (506.945, 516.416)), (506.867, 516.2))), 
        Path(BezierCurve(((506.789, 515.984), (506.68402, 515.798)), (506.552, 515.642))), 
        Path(Line(496.67, 502.502)), 
        Path(Line(506.642, 502.502)), 
        Path(Line(506.642, 500.0)), 
        Path(Line(492.90802, 500.0)), 
        Path(Line(492.90802, 501.332)), 
        Path(BezierCurve(((492.90802, 501.488), (492.944, 501.671)), (493.01602, 501.881))), 
        Path(BezierCurve(((493.088, 502.091), (493.196, 502.292)), (493.34003, 502.484))), 
        Path(Line(503.276, 515.714)), 
        Path(Line(493.466, 515.714)), 
        Path(Line(493.466, 518.234)), 
        Path(Line(506.984, 518.234)), 
        Path(Line(506.984, 516.866)), 
        Path(ClosePath)
    ];

    // Range
    let z_top       = 500.0;
    let z_bottom    = 518.234;

    // Renders near the center of a 1000,1000 canvas
    let mut instructions = vec![];

    instructions.canvas_height(1000.0);
    instructions.center_region(0.0, 0.0, 1000.0, 1000.0);
    instructions.extend(letter_z);
    instructions.fill_color(Color::Rgba(0.0, 0.0, 0.0, 1.0));
    instructions.fill();

    // This test uses the fixed 1080 pixel height window, but moves pixels around the top and the bottom
    let transform   = ScanlineTransform::for_region(&(-1.0..1.0), 1080);

    // Drawing is in the range 0-1000 but we'll have 1080 pixels
    let z_top       = (z_top/1000.0)*1080.0;
    let z_bottom    = (z_bottom/1000.0)*1080.0;

    // Record the locations where we do and do not find a match against the 'z'
    let mut succeeded   = vec![];
    let mut failed      = vec![];

    for offset in 0..100 {
        // Search around the pixel boundary for the bottom/top of the z
        let offset  = (offset as f64) / 100.0;
        let offset  = -0.45 + (offset * 0.9);

        let pos1    = z_top + offset;
        let pos2    = z_bottom + offset;
        let pos1    = -transform.fractional_pixel_x_to_source_x(pos1);
        let pos2    = -transform.fractional_pixel_x_to_source_x(pos2);

        // Plan at the positions
        let plan1 = plan_layer_0_line_on_drawing(instructions.clone(), pos1);
        let plan2 = plan_layer_0_line_on_drawing(instructions.clone(), pos2);

        // Top or bottom of the z should hit both sides
        if plan1.spans().len() > 0 {
            succeeded.push(pos1);
        } else {
            failed.push(pos1);
        }

        if plan2.spans().len() > 0 {
            succeeded.push(pos2);
        } else {
            failed.push(pos2);
        }
    }

    assert!(failed.is_empty(), "{}/{} failed\n\nFailed={:?}", failed.len(), failed.len() + succeeded.len(), failed);
}

#[test]
fn lower_edges_7() {
    // This letter 'z' has an issue drawing its last line: it shimmers in and out of existence as the pixel height changes
    use Draw::*;
    use PathOp::*;

    let letter_z = vec![
        Path(Move(506.984, 516.866)), 
        Path(BezierCurve(((506.984, 516.638), (506.945, 516.416)), (506.867, 516.2))), 
        Path(BezierCurve(((506.789, 515.984), (506.68402, 515.798)), (506.552, 515.642))), 
        Path(Line(496.67, 502.502)), 
        Path(Line(506.642, 502.502)), 
        Path(Line(506.642, 500.0)), 
        Path(Line(492.90802, 500.0)), 
        Path(Line(492.90802, 501.332)), 
        Path(BezierCurve(((492.90802, 501.488), (492.944, 501.671)), (493.01602, 501.881))), 
        Path(BezierCurve(((493.088, 502.091), (493.196, 502.292)), (493.34003, 502.484))), 
        Path(Line(503.276, 515.714)), 
        Path(Line(493.466, 515.714)), 
        Path(Line(493.466, 518.234)), 
        Path(Line(506.984, 518.234)), 
        Path(Line(506.984, 516.866)), 
        Path(ClosePath)
    ];

    // Range
    let z_top       = 500.0;
    let z_bottom    = 518.234;

    // Renders near the center of a 1000,1000 canvas
    let mut instructions = vec![];

    instructions.canvas_height(1000.0);
    instructions.center_region(0.0, 0.0, 1000.0, 1000.0);
    instructions.extend(letter_z);
    instructions.fill_color(Color::Rgba(0.0, 0.0, 0.0, 1.0));
    instructions.fill();

    // Record the locations where we do and do not find a match against the 'z'
    let mut succeeded   = vec![];
    let mut failed      = vec![];

    for offset in 0..100 {
        // This test modifies the height of the region that we're rendering
        let offset      = offset as f64;
        let offset      = offset - 50.0;
        let height      = 1080.0 + offset;
        let transform   = ScanlineTransform::for_region(&(-1.0..1.0), height as _);

        // Drawing is in the range 0-1000 but we'll have 1080 pixels
        let z_top       = (z_top/1000.0)*height;
        let z_bottom    = (z_bottom/1000.0)*height;

        // Search at the boundary of the lower part of the z, on around an actual pixel boundary
        let pos1    = -transform.fractional_pixel_x_to_source_x((z_top+0.5).floor());
        let pos2    = -transform.fractional_pixel_x_to_source_x((z_bottom+0.5).floor());

        let upper1 = ((transform.source_x_to_pixels(-pos1)+0.5)/height)*1000.0;
        let lower1 = ((transform.source_x_to_pixels(-pos1)-0.5)/height)*1000.0;

        let upper2 = ((transform.source_x_to_pixels(-pos2)+0.5)/height)*1000.0;
        let lower2 = ((transform.source_x_to_pixels(-pos2)-0.5)/height)*1000.0;

        assert!(upper2.max(lower2) >= 518.234 && upper2.min(lower2) <= 518.234, "{}..{}", lower2, upper2);
        assert!(upper1.max(lower1) >= 500.0 && upper1.min(lower1) <= 500.0, "{}..{}", lower1, upper1);

        // Plan at the positions
        let plan1 = plan_layer_0_line_on_drawing_with_height(instructions.clone(), pos1, height);
        let plan2 = plan_layer_0_line_on_drawing_with_height(instructions.clone(), pos2, height);

        // Top or bottom of the z should hit both sides
        if plan1.spans().len() > 0 {
            succeeded.push(height);
        } else {
            failed.push(height);
        }

        if plan2.spans().len() > 0 {
            succeeded.push(height);
        } else {
            failed.push(height);
        }
    }

    assert!(failed.is_empty(), "{}/{} failed\n\nFailed={:?}", failed.len(), failed.len() + succeeded.len(), failed);
}

#[test]
fn lower_edges_8() {
    // Same as for lower_edges_7 but the 'top' of the 'z' is moved to 499.999 instead of 500.0
    // (This passes where lower_edges_7 would fail, and also doesn't exhibit the behaviour in the real app)
    use Draw::*;
    use PathOp::*;

    let letter_z = vec![
        Path(Move(506.984, 516.866)), 
        Path(BezierCurve(((506.984, 516.638), (506.945, 516.416)), (506.867, 516.2))), 
        Path(BezierCurve(((506.789, 515.984), (506.68402, 515.798)), (506.552, 515.642))), 
        Path(Line(496.67, 502.502)), 
        Path(Line(506.642, 502.502)), 
        Path(Line(506.642, 499.999)), 
        Path(Line(492.90802, 499.999)), 
        Path(Line(492.90802, 501.332)), 
        Path(BezierCurve(((492.90802, 501.488), (492.944, 501.671)), (493.01602, 501.881))), 
        Path(BezierCurve(((493.088, 502.091), (493.196, 502.292)), (493.34003, 502.484))), 
        Path(Line(503.276, 515.714)), 
        Path(Line(493.466, 515.714)), 
        Path(Line(493.466, 518.234)), 
        Path(Line(506.984, 518.234)), 
        Path(Line(506.984, 516.866)), 
        Path(ClosePath)
    ];

    // Range
    let z_top       = 499.999;
    let z_bottom    = 518.234;

    // Renders near the center of a 1000,1000 canvas
    let mut instructions = vec![];

    instructions.canvas_height(1000.0);
    instructions.center_region(0.0, 0.0, 1000.0, 1000.0);
    instructions.extend(letter_z);
    instructions.fill_color(Color::Rgba(0.0, 0.0, 0.0, 1.0));
    instructions.fill();

    // Record the locations where we do and do not find a match against the 'z'
    let mut succeeded   = vec![];
    let mut failed      = vec![];

    for offset in 0..100 {
        // This test modifies the height of the region that we're rendering
        let offset      = offset as f64;
        let offset      = offset - 50.0;
        let height      = 1080.0 + offset;
        let transform   = ScanlineTransform::for_region(&(-1.0..1.0), height as _);

        // Drawing is in the range 0-1000 but we'll have 1080 pixels
        let z_top       = (z_top/1000.0)*height;
        let z_bottom    = (z_bottom/1000.0)*height;

        // Search at the boundary of the lower part of the z, on around an actual pixel boundary
        let pos1    = -transform.fractional_pixel_x_to_source_x((z_top+0.5).floor());
        let pos2    = -transform.fractional_pixel_x_to_source_x((z_bottom+0.5).floor());

        // Plan at the positions
        let plan1 = plan_layer_0_line_on_drawing_with_height(instructions.clone(), pos1, height);
        let plan2 = plan_layer_0_line_on_drawing_with_height(instructions.clone(), pos2, height);

        // Top or bottom of the z should hit both sides
        if plan1.spans().len() > 0 {
            succeeded.push(height);
        } else {
            failed.push(height);
        }

        if plan2.spans().len() > 0 {
            succeeded.push(height);
        } else {
            failed.push(height);
        }
    }

    assert!(failed.is_empty(), "{}/{} failed\n\nFailed={:?}", failed.len(), failed.len() + succeeded.len(), failed);
}
