use flo_render_software::edgeplan::*;
use flo_render_software::edges::*;
use flo_render_software::scanplan::*;
use flo_render_software::*;

#[test]
fn simple_rectangle() {
    // Declare a program cache for the rectangle
    let mut program_cache   = PixelProgramCache::empty();
    let program_id          = program_cache.add_program(PerPixelProgramFn::from(|_x, _y, _data: &()| 12.0f64));
    let mut data_cache      = program_cache.create_data_cache();
    let program_data_id     = program_cache.store_program_data(&program_id, &mut data_cache, ());

    let rectangle_shape = ShapeId::new();
    let rectangle_edge  = RectangleEdge::new(rectangle_shape, 100.0..200.0, 125.0..175.0);
    let edge_plan       = EdgePlan::new().with_shape_description(rectangle_shape, ShapeDescriptor::opaque(program_data_id)).with_edge(rectangle_edge);

    let pixel_plan      = plan_pixel_scanlines(&edge_plan, &[124.0, 125.0, 126.0], 0..1000);
    assert!(pixel_plan.len() == 3);

    assert!(pixel_plan[0].iter_as_spans().count() == 0, "[0, y == 124.0] {} != 0", pixel_plan[0].iter_as_spans().count());
    assert!(pixel_plan[1].iter_as_spans().count() == 1, "[1, y == 125.0] {} != 1", pixel_plan[1].iter_as_spans().count());
    assert!(pixel_plan[2].iter_as_spans().count() == 1, "[2, y == 126.0] {} != 1", pixel_plan[2].iter_as_spans().count());

    assert!(pixel_plan[1].iter_as_spans().collect::<Vec<_>>() == vec![ScanSpan::opaque(100..200, program_data_id)], "[1, y == 125.0] {:?}", pixel_plan[1].iter_as_spans().collect::<Vec<_>>());
    assert!(pixel_plan[2].iter_as_spans().collect::<Vec<_>>() == vec![ScanSpan::opaque(100..200, program_data_id)], "[2, y == 126.0] {:?}", pixel_plan[1].iter_as_spans().collect::<Vec<_>>());
}

#[test]
fn small_rectangle_on_rectangle() {
    // Declare a program cache for the rectangle
    let mut program_cache   = PixelProgramCache::empty();
    let program_id          = program_cache.add_program(PerPixelProgramFn::from(|_x, _y, _data: &()| 12.0f64));
    let mut data_cache      = program_cache.create_data_cache();
    let program_data_id_1   = program_cache.store_program_data(&program_id, &mut data_cache, ());
    let program_data_id_2   = program_cache.store_program_data(&program_id, &mut data_cache, ());

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

    let pixel_plan = plan_pixel_scanlines(&edge_plan, &[139.0, 140.0, 141.0], 0..1000);
    assert!(pixel_plan.len() == 3);

    assert!(pixel_plan[0].iter_as_spans().count() == 1, "[0, y == 139.0] {} != 1", pixel_plan[0].iter_as_spans().count());
    assert!(pixel_plan[1].iter_as_spans().count() == 3, "[1, y == 140.0] {} != 3", pixel_plan[1].iter_as_spans().count());
    assert!(pixel_plan[2].iter_as_spans().count() == 3, "[2, y == 141.0] {} != 3", pixel_plan[2].iter_as_spans().count());

    assert!(pixel_plan[0].iter_as_spans().collect::<Vec<_>>() == vec![ScanSpan::opaque(100..200, program_data_id_1)], "[1, y == 139.0] {:?}", pixel_plan[1].iter_as_spans().collect::<Vec<_>>());
    assert!(pixel_plan[1].iter_as_spans().collect::<Vec<_>>() == vec![ScanSpan::opaque(100..140, program_data_id_1), ScanSpan::opaque(140..160, program_data_id_2), ScanSpan::opaque(160..200, program_data_id_1)], "[1, y == 140.0] {:?}", pixel_plan[1].iter_as_spans().collect::<Vec<_>>());
    assert!(pixel_plan[2].iter_as_spans().collect::<Vec<_>>() == vec![ScanSpan::opaque(100..140, program_data_id_1), ScanSpan::opaque(140..160, program_data_id_2), ScanSpan::opaque(160..200, program_data_id_1)], "[2, y == 141.0] {:?}", pixel_plan[2].iter_as_spans().collect::<Vec<_>>());
}

#[test]
fn small_rectangle_under_rectangle() {
    // Declare a program cache for the rectangle
    let mut program_cache   = PixelProgramCache::empty();
    let program_id          = program_cache.add_program(PerPixelProgramFn::from(|_x, _y, _data: &()| 12.0f64));
    let mut data_cache      = program_cache.create_data_cache();
    let program_data_id_1   = program_cache.store_program_data(&program_id, &mut data_cache, ());
    let program_data_id_2   = program_cache.store_program_data(&program_id, &mut data_cache, ());

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

    let pixel_plan = plan_pixel_scanlines(&edge_plan, &[139.0, 140.0, 141.0], 0..1000);
    assert!(pixel_plan.len() == 3);

    assert!(pixel_plan[0].iter_as_spans().count() == 1, "[0, y == 139.0] {} != 1 ({:?})", pixel_plan[0].iter_as_spans().count(), pixel_plan[1].iter_as_spans().collect::<Vec<_>>());
    assert!(pixel_plan[1].iter_as_spans().count() == 1, "[1, y == 140.0] {} != 1 ({:?})", pixel_plan[1].iter_as_spans().count(), pixel_plan[1].iter_as_spans().collect::<Vec<_>>());
    assert!(pixel_plan[2].iter_as_spans().count() == 1, "[2, y == 141.0] {} != 1 ({:?})", pixel_plan[2].iter_as_spans().count(), pixel_plan[1].iter_as_spans().collect::<Vec<_>>());

    assert!(pixel_plan[0].iter_as_spans().collect::<Vec<_>>() == vec![ScanSpan::opaque(100..200, program_data_id_1)], "[1, y == 139.0] {:?}", pixel_plan[1].iter_as_spans().collect::<Vec<_>>());
    assert!(pixel_plan[1].iter_as_spans().collect::<Vec<_>>() == vec![ScanSpan::opaque(100..200, program_data_id_1)], "[1, y == 140.0] {:?}", pixel_plan[1].iter_as_spans().collect::<Vec<_>>());
    assert!(pixel_plan[2].iter_as_spans().collect::<Vec<_>>() == vec![ScanSpan::opaque(100..200, program_data_id_1)], "[2, y == 141.0] {:?}", pixel_plan[2].iter_as_spans().collect::<Vec<_>>());
}

#[test]
fn transparent_rectangle_on_rectangle() {
    // Declare a program cache for the rectangle
    let mut program_cache   = PixelProgramCache::empty();
    let program_id          = program_cache.add_program(PerPixelProgramFn::from(|_x, _y, _data: &()| 12.0f64));
    let mut data_cache      = program_cache.create_data_cache();
    let program_data_id_1   = program_cache.store_program_data(&program_id, &mut data_cache, ());
    let program_data_id_2   = program_cache.store_program_data(&program_id, &mut data_cache, ());

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

    let pixel_plan = plan_pixel_scanlines(&edge_plan, &[139.0, 140.0, 141.0], 0..1000);
    assert!(pixel_plan.len() == 3);

    assert!(pixel_plan[0].iter_as_spans().count() == 1, "[0, y == 139.0] {} != 1", pixel_plan[0].iter_as_spans().count());
    assert!(pixel_plan[1].iter_as_spans().count() == 4, "[1, y == 140.0] {} != 4", pixel_plan[1].iter_as_spans().count());
    assert!(pixel_plan[2].iter_as_spans().count() == 4, "[2, y == 141.0] {} != 4", pixel_plan[2].iter_as_spans().count());

    assert!(pixel_plan[0].iter_as_spans().collect::<Vec<_>>() == vec![ScanSpan::opaque(100..200, program_data_id_1)], "[1, y == 139.0] {:?}", pixel_plan[1].iter_as_spans().collect::<Vec<_>>());
    assert!(pixel_plan[1].iter_as_spans().collect::<Vec<_>>() == vec![ScanSpan::opaque(100..140, program_data_id_1), ScanSpan::opaque(140..160, program_data_id_1), ScanSpan::transparent(140..160, program_data_id_2), ScanSpan::opaque(160..200, program_data_id_1)], "[1, y == 140.0] {:?}", pixel_plan[1].iter_as_spans().collect::<Vec<_>>());
    assert!(pixel_plan[2].iter_as_spans().collect::<Vec<_>>() == vec![ScanSpan::opaque(100..140, program_data_id_1), ScanSpan::opaque(140..160, program_data_id_1), ScanSpan::transparent(140..160, program_data_id_2), ScanSpan::opaque(160..200, program_data_id_1)], "[2, y == 141.0] {:?}", pixel_plan[2].iter_as_spans().collect::<Vec<_>>());
}
