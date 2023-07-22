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
    assert!(spans == vec![ScanSpan::opaque(0..100, scanline_data_id)]);
}
