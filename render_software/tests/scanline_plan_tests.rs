use flo_render_software::*;
use flo_render_software::scanplan::*;

#[test]
fn add_first_scanline() {
    // Create a data token for the scanline we're generating
    let mut program_cache = PixelProgramCache::empty();
    let program_id          = program_cache.add_program(PerPixelProgramFn::from(|_x, _y, _data: &()| 12.0f64));
    let mut data_cache      = program_cache.create_data_cache();
    let program_data_id     = program_cache.store_program_data(&program_id, &mut data_cache, ());
    let scanline_data_id    = program_cache.create_scanline_data(&mut data_cache, 0, &vec![], program_data_id);

    // Set up a plan for a scanline using this program
    let mut plan = ScanlinePlan::new();
    plan.add_span(ScanSpan::opaque(0..100, scanline_data_id));

    // Read the span back again
    let spans = plan.iter_as_spans().collect::<Vec<_>>();
    assert!(spans == vec![ScanSpan::opaque(0..100, scanline_data_id)], "Unexpected spans: {:?}", spans);
}

#[test]
fn add_two_scanlines() {
    // Create a data token for the scanline we're generating
    let mut program_cache = PixelProgramCache::empty();
    let program_id          = program_cache.add_program(PerPixelProgramFn::from(|_x, _y, _data: &()| 12.0f64));
    let mut data_cache      = program_cache.create_data_cache();
    let program_data_id     = program_cache.store_program_data(&program_id, &mut data_cache, ());
    let scanline_data_id_1  = program_cache.create_scanline_data(&mut data_cache, 0, &vec![], program_data_id);
    let scanline_data_id_2  = program_cache.create_scanline_data(&mut data_cache, 0, &vec![], program_data_id);

    // Set up a plan for a scanline using this program (two spans)
    let mut plan = ScanlinePlan::new();
    plan.add_span(ScanSpan::opaque(0..100, scanline_data_id_1));
    plan.add_span(ScanSpan::opaque(200..300, scanline_data_id_2));

    // Read the span back again
    let spans = plan.iter_as_spans().collect::<Vec<_>>();
    assert!(spans == vec![ScanSpan::opaque(0..100, scanline_data_id_1), ScanSpan::opaque(200..300, scanline_data_id_2)], "Unexpected spans: {:?}", spans);
}

#[test]
fn add_two_scanlines_reverse() {
    // Create a data token for the scanline we're generating
    let mut program_cache = PixelProgramCache::empty();
    let program_id          = program_cache.add_program(PerPixelProgramFn::from(|_x, _y, _data: &()| 12.0f64));
    let mut data_cache      = program_cache.create_data_cache();
    let program_data_id     = program_cache.store_program_data(&program_id, &mut data_cache, ());
    let scanline_data_id    = program_cache.create_scanline_data(&mut data_cache, 0, &vec![], program_data_id);

    // Set up a plan for a scanline using this program (two spans, reverse order of above)
    let mut plan = ScanlinePlan::new();
    plan.add_span(ScanSpan::opaque(200..300, scanline_data_id));
    plan.add_span(ScanSpan::opaque(0..100, scanline_data_id));

    // Read the span back again
    let spans = plan.iter_as_spans().collect::<Vec<_>>();
    assert!(spans == vec![ScanSpan::opaque(0..100, scanline_data_id), ScanSpan::opaque(200..300, scanline_data_id)], "Unexpected spans: {:?}", spans);
}

#[test]
fn add_opaque_scanline_start() {
    // Create a data token for the scanline we're generating
    let mut program_cache = PixelProgramCache::empty();
    let program_id          = program_cache.add_program(PerPixelProgramFn::from(|_x, _y, _data: &()| 12.0f64));
    let mut data_cache      = program_cache.create_data_cache();
    let program_data_id     = program_cache.store_program_data(&program_id, &mut data_cache, ());
    let scanline_data_id_1  = program_cache.create_scanline_data(&mut data_cache, 0, &vec![], program_data_id);
    let scanline_data_id_2  = program_cache.create_scanline_data(&mut data_cache, 0, &vec![], program_data_id);

    // Set up a plan for a scanline using this program (here we split a span with another program)
    let mut plan = ScanlinePlan::new();
    plan.add_span(ScanSpan::opaque(25..100, scanline_data_id_1));
    plan.add_span(ScanSpan::opaque(0..50, scanline_data_id_2));

    // Read the span back again (TODO: this should actually produce a continguous span rather than splitting the original)
    let spans = plan.iter_as_spans().collect::<Vec<_>>();
    assert!(spans == vec![ScanSpan::opaque(0..25, scanline_data_id_2), ScanSpan::opaque(25..50, scanline_data_id_2), ScanSpan::opaque(50..100, scanline_data_id_1)], "Unexpected spans: {:?}", spans);
}

#[test]
fn add_opaque_scanline_end() {
    // Create a data token for the scanline we're generating
    let mut program_cache = PixelProgramCache::empty();
    let program_id          = program_cache.add_program(PerPixelProgramFn::from(|_x, _y, _data: &()| 12.0f64));
    let mut data_cache      = program_cache.create_data_cache();
    let program_data_id     = program_cache.store_program_data(&program_id, &mut data_cache, ());
    let scanline_data_id_1  = program_cache.create_scanline_data(&mut data_cache, 0, &vec![], program_data_id);
    let scanline_data_id_2  = program_cache.create_scanline_data(&mut data_cache, 0, &vec![], program_data_id);

    // Set up a plan for a scanline using this program (here we split a span with another program)
    let mut plan = ScanlinePlan::new();
    plan.add_span(ScanSpan::opaque(0..75, scanline_data_id_1));
    plan.add_span(ScanSpan::opaque(50..100, scanline_data_id_2));

    // Read the span back again (TODO: this should actually produce a continguous span rather than splitting the original)
    let spans = plan.iter_as_spans().collect::<Vec<_>>();
    assert!(spans == vec![ScanSpan::opaque(0..50, scanline_data_id_1), ScanSpan::opaque(50..75, scanline_data_id_2), ScanSpan::opaque(75..100, scanline_data_id_2)], "Unexpected spans: {:?}", spans);
}

#[test]
fn add_opaque_scanline_middle() {
    // Create a data token for the scanline we're generating
    let mut program_cache = PixelProgramCache::empty();
    let program_id          = program_cache.add_program(PerPixelProgramFn::from(|_x, _y, _data: &()| 12.0f64));
    let mut data_cache      = program_cache.create_data_cache();
    let program_data_id     = program_cache.store_program_data(&program_id, &mut data_cache, ());
    let scanline_data_id_1  = program_cache.create_scanline_data(&mut data_cache, 0, &vec![], program_data_id);
    let scanline_data_id_2  = program_cache.create_scanline_data(&mut data_cache, 0, &vec![], program_data_id);

    // Set up a plan for a scanline using this program (here we split a span with another program)
    let mut plan = ScanlinePlan::new();
    plan.add_span(ScanSpan::opaque(0..100, scanline_data_id_1));
    plan.add_span(ScanSpan::opaque(25..75, scanline_data_id_2));

    // Read the span back again
    let spans = plan.iter_as_spans().collect::<Vec<_>>();
    assert!(spans == vec![ScanSpan::opaque(0..25, scanline_data_id_1), ScanSpan::opaque(25..75, scanline_data_id_2), ScanSpan::opaque(75..100, scanline_data_id_1)], "Unexpected spans: {:?}", spans);
}

#[test]
fn add_transparent_scanline_middle() {
    // Create a data token for the scanline we're generating
    let mut program_cache = PixelProgramCache::empty();
    let program_id          = program_cache.add_program(PerPixelProgramFn::from(|_x, _y, _data: &()| 12.0f64));
    let mut data_cache      = program_cache.create_data_cache();
    let program_data_id     = program_cache.store_program_data(&program_id, &mut data_cache, ());
    let scanline_data_id_1  = program_cache.create_scanline_data(&mut data_cache, 0, &vec![], program_data_id);
    let scanline_data_id_2  = program_cache.create_scanline_data(&mut data_cache, 0, &vec![], program_data_id);

    // Set up a plan for a scanline using this program (here we split a span with another program: in this case a transparent one so both programs need to run over that range)
    let mut plan = ScanlinePlan::new();
    plan.add_span(ScanSpan::opaque(0..100, scanline_data_id_1));
    plan.add_span(ScanSpan::transparent(25..75, scanline_data_id_2));

    // Read the span back again
    let spans = plan.iter_as_spans().collect::<Vec<_>>();
    assert!(spans == vec![ScanSpan::opaque(0..25, scanline_data_id_1), ScanSpan::opaque(25..75, scanline_data_id_1), ScanSpan::opaque(25..75, scanline_data_id_2), ScanSpan::opaque(75..100, scanline_data_id_1)], "Unexpected spans: {:?}", spans);
}
