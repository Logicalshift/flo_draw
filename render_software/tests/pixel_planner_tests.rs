use flo_render_software::edgeplan::*;
use flo_render_software::edges::*;
use flo_render_software::pixel::*;
use flo_render_software::scanplan::*;

fn strip_y_coordinates(with_coordinates: Vec<(f64, ScanlinePlan)>) -> Vec<ScanlinePlan> {
    with_coordinates.into_iter()
        .map(|(_, plan)| plan)
        .collect()
}

#[test]
fn simple_rectangle() {
    // The program data ID usually maps to the program cache (specifies what to do in a particular span)
    let program_data_id = PixelProgramDataId(0);

    let rectangle_shape = ShapeId::new();
    let rectangle_edge  = RectangleEdge::new(rectangle_shape, 100.0..200.0, 125.0..175.0);
    let edge_plan       = EdgePlan::new().with_shape_description(rectangle_shape, ShapeDescriptor::opaque(program_data_id)).with_edge(rectangle_edge);

    let pixel_plan      = strip_y_coordinates(PixelScanPlanner::plan(&edge_plan, &ScanlineTransform::identity(), &[124.0, 125.0, 126.0], 0.0..1000.0));
    assert!(pixel_plan.len() == 3);

    assert!(pixel_plan[0].iter_as_spans().count() == 0, "[0, y == 124.0] {} != 0", pixel_plan[0].iter_as_spans().count());
    assert!(pixel_plan[1].iter_as_spans().count() == 1, "[1, y == 125.0] {} != 1", pixel_plan[1].iter_as_spans().count());
    assert!(pixel_plan[2].iter_as_spans().count() == 1, "[2, y == 126.0] {} != 1", pixel_plan[2].iter_as_spans().count());

    assert!(pixel_plan[1].iter_as_spans().collect::<Vec<_>>() == vec![ScanSpan::opaque(100.0..200.0, program_data_id)], "[1, y == 125.0] {:?}", pixel_plan[1].iter_as_spans().collect::<Vec<_>>());
    assert!(pixel_plan[2].iter_as_spans().collect::<Vec<_>>() == vec![ScanSpan::opaque(100.0..200.0, program_data_id)], "[2, y == 126.0] {:?}", pixel_plan[1].iter_as_spans().collect::<Vec<_>>());
}

#[test]
fn simple_rectangle_canvas_coords() {
    // The program data ID usually maps to the program cache (specifies what to do in a particular span)
    let program_data_id = PixelProgramDataId(0);

    let rectangle_shape = ShapeId::new();
    let rectangle_edge  = RectangleEdge::new(rectangle_shape, -0.5..0.25, -0.4..0.4);
    let edge_plan       = EdgePlan::new().with_shape_description(rectangle_shape, ShapeDescriptor::opaque(program_data_id)).with_edge(rectangle_edge);

    let pixel_plan      = strip_y_coordinates(PixelScanPlanner::plan(&edge_plan, &ScanlineTransform::identity(), &[-0.6, -0.3, 0.1], -1.0..1.0));
    assert!(pixel_plan.len() == 3);

    assert!(pixel_plan[0].iter_as_spans().count() == 0, "[0, y == -0.6] {} != 0", pixel_plan[0].iter_as_spans().count());
    assert!(pixel_plan[1].iter_as_spans().count() == 1, "[1, y == -0.3] {} != 1", pixel_plan[1].iter_as_spans().count());
    assert!(pixel_plan[2].iter_as_spans().count() == 1, "[2, y == 0.1] {} != 1", pixel_plan[2].iter_as_spans().count());

    assert!(pixel_plan[1].iter_as_spans().collect::<Vec<_>>() == vec![ScanSpan::opaque(-0.5..0.25, program_data_id)], "[1, y == -0.3] {:?}", pixel_plan[1].iter_as_spans().collect::<Vec<_>>());
    assert!(pixel_plan[2].iter_as_spans().collect::<Vec<_>>() == vec![ScanSpan::opaque(-0.5..0.25, program_data_id)], "[2, y == 0.1] {:?}", pixel_plan[1].iter_as_spans().collect::<Vec<_>>());
}

#[test]
fn small_rectangle_on_rectangle() {
    // The program data ID usually maps to the program cache (specifies what to do in a particular span)
    let program_data_id_1   = PixelProgramDataId(1);
    let program_data_id_2   = PixelProgramDataId(2);

    // One rectangle in front of another
    let rectangle_shape_1   = ShapeId::new();
    let rectangle_shape_2   = ShapeId::new();
    let rectangle_edge_1    = RectangleEdge::new(rectangle_shape_1, 100.0..200.0, 125.0..175.0);
    let rectangle_edge_2    = RectangleEdge::new(rectangle_shape_2, 140.0..160.0, 140.0..160.0);
    let edge_plan           = EdgePlan::new()
        .with_shape_description(rectangle_shape_1, ShapeDescriptor::opaque(program_data_id_1).with_z_index(0))
        .with_shape_description(rectangle_shape_2, ShapeDescriptor::opaque(program_data_id_2).with_z_index(1))
        .with_edge(rectangle_edge_1)
        .with_edge(rectangle_edge_2);

    let pixel_plan = strip_y_coordinates(PixelScanPlanner::plan(&edge_plan, &ScanlineTransform::identity(), &[139.0, 140.0, 141.0], 0.0..1000.0));
    assert!(pixel_plan.len() == 3);

    assert!(pixel_plan[0].iter_as_spans().count() == 1, "[0, y == 139.0] {} != 1", pixel_plan[0].iter_as_spans().count());
    assert!(pixel_plan[1].iter_as_spans().count() == 3, "[1, y == 140.0] {} != 3", pixel_plan[1].iter_as_spans().count());
    assert!(pixel_plan[2].iter_as_spans().count() == 3, "[2, y == 141.0] {} != 3", pixel_plan[2].iter_as_spans().count());

    assert!(pixel_plan[0].iter_as_spans().collect::<Vec<_>>() == vec![ScanSpan::opaque(100.0..200.0, program_data_id_1)], "[0, y == 139.0] {:?}", pixel_plan[1].iter_as_spans().collect::<Vec<_>>());
    assert!(pixel_plan[1].iter_as_spans().collect::<Vec<_>>() == vec![ScanSpan::opaque(100.0..140.0, program_data_id_1), ScanSpan::opaque(140.0..160.0, program_data_id_2), ScanSpan::opaque(160.0..200.0, program_data_id_1)], "[1, y == 140.0] {:?}", pixel_plan[1].iter_as_spans().collect::<Vec<_>>());
    assert!(pixel_plan[2].iter_as_spans().collect::<Vec<_>>() == vec![ScanSpan::opaque(100.0..140.0, program_data_id_1), ScanSpan::opaque(140.0..160.0, program_data_id_2), ScanSpan::opaque(160.0..200.0, program_data_id_1)], "[2, y == 141.0] {:?}", pixel_plan[2].iter_as_spans().collect::<Vec<_>>());
}

#[test]
fn identical_overlapping_rectangles_1() {
    // The program data ID usually maps to the program cache (specifies what to do in a particular span)
    let program_data_id_1   = PixelProgramDataId(1);
    let program_data_id_2   = PixelProgramDataId(2);

    // One rectangle in front of another
    let rectangle_shape_1   = ShapeId::new();
    let rectangle_shape_2   = ShapeId::new();
    let rectangle_edge_1    = RectangleEdge::new(rectangle_shape_1, 100.0..200.0, 125.0..175.0);
    let rectangle_edge_2    = RectangleEdge::new(rectangle_shape_2, 100.0..200.0, 125.0..175.0);
    let edge_plan           = EdgePlan::new()
        .with_shape_description(rectangle_shape_1, ShapeDescriptor::opaque(program_data_id_1).with_z_index(0))
        .with_shape_description(rectangle_shape_2, ShapeDescriptor::opaque(program_data_id_2).with_z_index(1))
        .with_edge(rectangle_edge_1)
        .with_edge(rectangle_edge_2);

    let pixel_plan = strip_y_coordinates(PixelScanPlanner::plan(&edge_plan, &ScanlineTransform::identity(), &[124.0, 125.0, 126.0], 0.0..1000.0));
    assert!(pixel_plan.len() == 3);

    assert!(pixel_plan[0].iter_as_spans().count() == 0, "[0, y == 124.0] {} != 0", pixel_plan[0].iter_as_spans().count());
    assert!(pixel_plan[1].iter_as_spans().count() == 1, "[1, y == 125.0] {} != 1", pixel_plan[1].iter_as_spans().count());
    assert!(pixel_plan[2].iter_as_spans().count() == 1, "[2, y == 126.0] {} != 1", pixel_plan[2].iter_as_spans().count());

    assert!(pixel_plan[0].iter_as_spans().collect::<Vec<_>>() == vec![], "[0, y == 124.0] {:?}", pixel_plan[1].iter_as_spans().collect::<Vec<_>>());
    assert!(pixel_plan[1].iter_as_spans().collect::<Vec<_>>() == vec![ScanSpan::opaque(100.0..200.0, program_data_id_2)], "[1, y == 125.0] {:?}", pixel_plan[1].iter_as_spans().collect::<Vec<_>>());
    assert!(pixel_plan[2].iter_as_spans().collect::<Vec<_>>() == vec![ScanSpan::opaque(100.0..200.0, program_data_id_2)], "[2, y == 126.0] {:?}", pixel_plan[2].iter_as_spans().collect::<Vec<_>>());
}

#[test]
fn identical_overlapping_rectangles_2() {
    // The program data ID usually maps to the program cache (specifies what to do in a particular span)
    let program_data_id_1   = PixelProgramDataId(1);
    let program_data_id_2   = PixelProgramDataId(2);

    // One rectangle in front of another
    let rectangle_shape_1   = ShapeId::new();
    let rectangle_shape_2   = ShapeId::new();
    let rectangle_edge_1    = RectangleEdge::new(rectangle_shape_1, 100.0..200.0, 125.0..175.0);
    let rectangle_edge_2    = RectangleEdge::new(rectangle_shape_2, 100.0..200.0, 125.0..175.0);
    let edge_plan           = EdgePlan::new()
        .with_shape_description(rectangle_shape_1, ShapeDescriptor::opaque(program_data_id_1).with_z_index(1))
        .with_shape_description(rectangle_shape_2, ShapeDescriptor::opaque(program_data_id_2).with_z_index(0))
        .with_edge(rectangle_edge_1)
        .with_edge(rectangle_edge_2);

    let pixel_plan = strip_y_coordinates(PixelScanPlanner::plan(&edge_plan, &ScanlineTransform::identity(), &[124.0, 125.0, 126.0], 0.0..1000.0));
    assert!(pixel_plan.len() == 3);

    assert!(pixel_plan[0].iter_as_spans().count() == 0, "[0, y == 124.0] {} != 0", pixel_plan[0].iter_as_spans().count());
    assert!(pixel_plan[1].iter_as_spans().count() == 1, "[1, y == 125.0] {} != 1", pixel_plan[1].iter_as_spans().count());
    assert!(pixel_plan[2].iter_as_spans().count() == 1, "[2, y == 126.0] {} != 1", pixel_plan[2].iter_as_spans().count());

    assert!(pixel_plan[0].iter_as_spans().collect::<Vec<_>>() == vec![], "[0, y == 124.0] {:?}", pixel_plan[1].iter_as_spans().collect::<Vec<_>>());
    assert!(pixel_plan[1].iter_as_spans().collect::<Vec<_>>() == vec![ScanSpan::opaque(100.0..200.0, program_data_id_1)], "[1, y == 125.0] {:?}", pixel_plan[1].iter_as_spans().collect::<Vec<_>>());
    assert!(pixel_plan[2].iter_as_spans().collect::<Vec<_>>() == vec![ScanSpan::opaque(100.0..200.0, program_data_id_1)], "[2, y == 126.0] {:?}", pixel_plan[2].iter_as_spans().collect::<Vec<_>>());
}

#[test]
fn identical_overlapping_rectangles_3() {
    // The program data ID usually maps to the program cache (specifies what to do in a particular span)
    let program_data_id_1   = PixelProgramDataId(1);
    let program_data_id_2   = PixelProgramDataId(2);

    // One rectangle in front of another
    let rectangle_shape_1   = ShapeId::new();
    let rectangle_shape_2   = ShapeId::new();
    let rectangle_edge_1    = RectangleEdge::new(rectangle_shape_1, 100.0..200.0, 125.0..175.0);
    let rectangle_edge_2    = RectangleEdge::new(rectangle_shape_2, 100.0..200.0, 125.0..175.0);
    let edge_plan           = EdgePlan::new()
        .with_shape_description(rectangle_shape_1, ShapeDescriptor::opaque(program_data_id_1).with_z_index(0))
        .with_shape_description(rectangle_shape_2, ShapeDescriptor::transparent(program_data_id_2).with_z_index(1))
        .with_edge(rectangle_edge_1)
        .with_edge(rectangle_edge_2);

    let pixel_plan = strip_y_coordinates(PixelScanPlanner::plan(&edge_plan, &ScanlineTransform::identity(), &[124.0, 125.0, 126.0], 0.0..1000.0));
    assert!(pixel_plan.len() == 3);

    assert!(pixel_plan[0].iter_as_spans().count() == 0, "[0, y == 124.0] {} != 0", pixel_plan[0].iter_as_spans().count());
    assert!(pixel_plan[1].iter_as_spans().count() == 2, "[1, y == 125.0] {} != 2", pixel_plan[1].iter_as_spans().count());
    assert!(pixel_plan[2].iter_as_spans().count() == 2, "[2, y == 126.0] {} != 2", pixel_plan[2].iter_as_spans().count());

    assert!(pixel_plan[0].iter_as_spans().collect::<Vec<_>>() == vec![], "[0, y == 124.0] {:?}", pixel_plan[1].iter_as_spans().collect::<Vec<_>>());
    assert!(pixel_plan[1].iter_as_spans().collect::<Vec<_>>() == vec![ScanSpan::opaque(100.0..200.0, program_data_id_1), ScanSpan::transparent(100.0..200.0, program_data_id_2)], "[1, y == 125.0] {:?}", pixel_plan[1].iter_as_spans().collect::<Vec<_>>());
    assert!(pixel_plan[2].iter_as_spans().collect::<Vec<_>>() == vec![ScanSpan::opaque(100.0..200.0, program_data_id_1), ScanSpan::transparent(100.0..200.0, program_data_id_2)], "[2, y == 126.0] {:?}", pixel_plan[2].iter_as_spans().collect::<Vec<_>>());
}

#[test]
fn small_rectangle_under_rectangle() {
    // The program data ID usually maps to the program cache (specifies what to do in a particular span)
    let program_data_id_1   = PixelProgramDataId(1);
    let program_data_id_2   = PixelProgramDataId(2);

    // One rectangle in front of another
    let rectangle_shape_1   = ShapeId::new();
    let rectangle_shape_2   = ShapeId::new();
    let rectangle_edge_1    = RectangleEdge::new(rectangle_shape_1, 100.0..200.0, 125.0..175.0);
    let rectangle_edge_2    = RectangleEdge::new(rectangle_shape_2, 140.0..160.0, 140.0..160.0);
    let edge_plan           = EdgePlan::new()
        .with_shape_description(rectangle_shape_1, ShapeDescriptor::opaque(program_data_id_1).with_z_index(1))
        .with_shape_description(rectangle_shape_2, ShapeDescriptor::opaque(program_data_id_2).with_z_index(0))
        .with_edge(rectangle_edge_1)
        .with_edge(rectangle_edge_2);

    let pixel_plan = strip_y_coordinates(PixelScanPlanner::plan(&edge_plan, &ScanlineTransform::identity(), &[139.0, 140.0, 141.0], 0.0..1000.0));
    assert!(pixel_plan.len() == 3);

    assert!(pixel_plan[0].iter_as_spans().count() == 1, "[0, y == 139.0] {} != 1 ({:?})", pixel_plan[0].iter_as_spans().count(), pixel_plan[1].iter_as_spans().collect::<Vec<_>>());
    assert!(pixel_plan[1].iter_as_spans().count() == 1, "[1, y == 140.0] {} != 1 ({:?})", pixel_plan[1].iter_as_spans().count(), pixel_plan[1].iter_as_spans().collect::<Vec<_>>());
    assert!(pixel_plan[2].iter_as_spans().count() == 1, "[2, y == 141.0] {} != 1 ({:?})", pixel_plan[2].iter_as_spans().count(), pixel_plan[1].iter_as_spans().collect::<Vec<_>>());

    assert!(pixel_plan[0].iter_as_spans().collect::<Vec<_>>() == vec![ScanSpan::opaque(100.0..200.0, program_data_id_1)], "[1, y == 139.0] {:?}", pixel_plan[1].iter_as_spans().collect::<Vec<_>>());
    assert!(pixel_plan[1].iter_as_spans().collect::<Vec<_>>() == vec![ScanSpan::opaque(100.0..200.0, program_data_id_1)], "[1, y == 140.0] {:?}", pixel_plan[1].iter_as_spans().collect::<Vec<_>>());
    assert!(pixel_plan[2].iter_as_spans().collect::<Vec<_>>() == vec![ScanSpan::opaque(100.0..200.0, program_data_id_1)], "[2, y == 141.0] {:?}", pixel_plan[2].iter_as_spans().collect::<Vec<_>>());
}

#[test]
fn transparent_rectangle_on_rectangle() {
    // The program data ID usually maps to the program cache (specifies what to do in a particular span)
    let program_data_id_1   = PixelProgramDataId(1);
    let program_data_id_2   = PixelProgramDataId(2);

    // One rectangle in front of another
    let rectangle_shape_1   = ShapeId::new();
    let rectangle_shape_2   = ShapeId::new();
    let rectangle_edge_1    = RectangleEdge::new(rectangle_shape_1, 100.0..200.0, 125.0..175.0);
    let rectangle_edge_2    = RectangleEdge::new(rectangle_shape_2, 140.0..160.0, 140.0..160.0);
    let edge_plan           = EdgePlan::new()
        .with_shape_description(rectangle_shape_1, ShapeDescriptor::opaque(program_data_id_1).with_z_index(0))
        .with_shape_description(rectangle_shape_2, ShapeDescriptor::transparent(program_data_id_2).with_z_index(1))
        .with_edge(rectangle_edge_1)
        .with_edge(rectangle_edge_2);

    let pixel_plan = strip_y_coordinates(PixelScanPlanner::plan(&edge_plan, &ScanlineTransform::identity(), &[139.0, 140.0, 141.0], 0.0..1000.0));
    assert!(pixel_plan.len() == 3);

    assert!(pixel_plan[0].iter_as_spans().count() == 1, "[0, y == 139.0] {} != 1", pixel_plan[0].iter_as_spans().count());
    assert!(pixel_plan[1].iter_as_spans().count() == 4, "[1, y == 140.0] {} != 4", pixel_plan[1].iter_as_spans().count());
    assert!(pixel_plan[2].iter_as_spans().count() == 4, "[2, y == 141.0] {} != 4", pixel_plan[2].iter_as_spans().count());

    assert!(pixel_plan[0].iter_as_spans().collect::<Vec<_>>() == vec![ScanSpan::opaque(100.0..200.0, program_data_id_1)], "[1, y == 139.0] {:?}", pixel_plan[1].iter_as_spans().collect::<Vec<_>>());
    assert!(pixel_plan[1].iter_as_spans().collect::<Vec<_>>() == vec![ScanSpan::opaque(100.0..140.0, program_data_id_1), ScanSpan::opaque(140.0..160.0, program_data_id_1), ScanSpan::transparent(140.0..160.0, program_data_id_2), ScanSpan::opaque(160.0..200.0, program_data_id_1)], "[1, y == 140.0] {:?}", pixel_plan[1].iter_as_spans().collect::<Vec<_>>());
    assert!(pixel_plan[2].iter_as_spans().collect::<Vec<_>>() == vec![ScanSpan::opaque(100.0..140.0, program_data_id_1), ScanSpan::opaque(140.0..160.0, program_data_id_1), ScanSpan::transparent(140.0..160.0, program_data_id_2), ScanSpan::opaque(160.0..200.0, program_data_id_1)], "[2, y == 141.0] {:?}", pixel_plan[2].iter_as_spans().collect::<Vec<_>>());
}

#[test]
fn clip_left() {
    // The program data ID usually maps to the program cache (specifies what to do in a particular span)
    let program_data_id = PixelProgramDataId(0);

    let rectangle_shape = ShapeId::new();
    let rectangle_edge  = RectangleEdge::new(rectangle_shape, -100.0..200.0, 125.0..175.0);
    let edge_plan       = EdgePlan::new().with_shape_description(rectangle_shape, ShapeDescriptor::opaque(program_data_id)).with_edge(rectangle_edge);

    let pixel_plan      = strip_y_coordinates(PixelScanPlanner::plan(&edge_plan, &ScanlineTransform::identity(), &[124.0, 125.0, 126.0], 0.0..1000.0));
    assert!(pixel_plan.len() == 3);

    assert!(pixel_plan[0].iter_as_spans().count() == 0, "[0, y == 124.0] {} != 0", pixel_plan[0].iter_as_spans().count());
    assert!(pixel_plan[1].iter_as_spans().count() == 1, "[1, y == 125.0] {} != 1", pixel_plan[1].iter_as_spans().count());
    assert!(pixel_plan[2].iter_as_spans().count() == 1, "[2, y == 126.0] {} != 1", pixel_plan[2].iter_as_spans().count());

    assert!(pixel_plan[1].iter_as_spans().collect::<Vec<_>>() == vec![ScanSpan::opaque(0.0..200.0, program_data_id)], "[1, y == 125.0] {:?}", pixel_plan[1].iter_as_spans().collect::<Vec<_>>());
    assert!(pixel_plan[2].iter_as_spans().collect::<Vec<_>>() == vec![ScanSpan::opaque(0.0..200.0, program_data_id)], "[2, y == 126.0] {:?}", pixel_plan[1].iter_as_spans().collect::<Vec<_>>());
}

#[test]
fn clip_right() {
    // The program data ID usually maps to the program cache (specifies what to do in a particular span)
    let program_data_id = PixelProgramDataId(0);

    let rectangle_shape = ShapeId::new();
    let rectangle_edge  = RectangleEdge::new(rectangle_shape, 100.0..1200.0, 125.0..175.0);
    let edge_plan       = EdgePlan::new().with_shape_description(rectangle_shape, ShapeDescriptor::opaque(program_data_id)).with_edge(rectangle_edge);

    let pixel_plan      = strip_y_coordinates(PixelScanPlanner::plan(&edge_plan, &ScanlineTransform::identity(), &[124.0, 125.0, 126.0], 0.0..1000.0));
    assert!(pixel_plan.len() == 3);

    assert!(pixel_plan[0].iter_as_spans().count() == 0, "[0, y == 124.0] {} != 0", pixel_plan[0].iter_as_spans().count());
    assert!(pixel_plan[1].iter_as_spans().count() == 1, "[1, y == 125.0] {} != 1", pixel_plan[1].iter_as_spans().count());
    assert!(pixel_plan[2].iter_as_spans().count() == 1, "[2, y == 126.0] {} != 1", pixel_plan[2].iter_as_spans().count());

    assert!(pixel_plan[1].iter_as_spans().collect::<Vec<_>>() == vec![ScanSpan::opaque(100.0..1000.0, program_data_id)], "[1, y == 125.0] {:?}", pixel_plan[1].iter_as_spans().collect::<Vec<_>>());
    assert!(pixel_plan[2].iter_as_spans().collect::<Vec<_>>() == vec![ScanSpan::opaque(100.0..1000.0, program_data_id)], "[2, y == 126.0] {:?}", pixel_plan[1].iter_as_spans().collect::<Vec<_>>());
}

#[test]
fn clip_both() {
    // The program data ID usually maps to the program cache (specifies what to do in a particular span)
    let program_data_id = PixelProgramDataId(0);

    let rectangle_shape = ShapeId::new();
    let rectangle_edge  = RectangleEdge::new(rectangle_shape, -100.0..1200.0, 125.0..175.0);
    let edge_plan       = EdgePlan::new().with_shape_description(rectangle_shape, ShapeDescriptor::opaque(program_data_id)).with_edge(rectangle_edge);

    let pixel_plan      = strip_y_coordinates(PixelScanPlanner::plan(&edge_plan, &ScanlineTransform::identity(), &[124.0, 125.0, 126.0], 0.0..1000.0));
    assert!(pixel_plan.len() == 3);

    assert!(pixel_plan[0].iter_as_spans().count() == 0, "[0, y == 124.0] {} != 0", pixel_plan[0].iter_as_spans().count());
    assert!(pixel_plan[1].iter_as_spans().count() == 1, "[1, y == 125.0] {} != 1", pixel_plan[1].iter_as_spans().count());
    assert!(pixel_plan[2].iter_as_spans().count() == 1, "[2, y == 126.0] {} != 1", pixel_plan[2].iter_as_spans().count());

    assert!(pixel_plan[1].iter_as_spans().collect::<Vec<_>>() == vec![ScanSpan::opaque(0.0..1000.0, program_data_id)], "[1, y == 125.0] {:?}", pixel_plan[1].iter_as_spans().collect::<Vec<_>>());
    assert!(pixel_plan[2].iter_as_spans().collect::<Vec<_>>() == vec![ScanSpan::opaque(0.0..1000.0, program_data_id)], "[2, y == 126.0] {:?}", pixel_plan[1].iter_as_spans().collect::<Vec<_>>());
}
