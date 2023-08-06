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
    let rectangle_edge  = RectangleEdge::new(rectangle_shape, 100.0..200.0, 150.0..200.0);
    let edge_plan       = EdgePlan::new().with_shape_description(rectangle_shape, ShapeDescriptor::opaque(program_data_id)).with_edge(rectangle_edge);

    let pixel_plan      = plan_pixel_scanlines(&edge_plan, &[99.0, 100.0, 101.0], 0..1000);
    assert!(pixel_plan.len() == 3);

    assert!(pixel_plan[0].iter_as_spans().count() == 0, "[0, y == 99.0] {} != 0", pixel_plan[0].iter_as_spans().count());
    assert!(pixel_plan[1].iter_as_spans().count() == 1, "[1, y == 100.0] {} != 1", pixel_plan[1].iter_as_spans().count());
    assert!(pixel_plan[2].iter_as_spans().count() == 1, "[2, y == 101.0] {} != 1", pixel_plan[2].iter_as_spans().count());
}
