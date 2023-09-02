use flo_render_software::draw::*;
use flo_render_software::pixel::*;
use flo_render_software::render::*;
use flo_render_software::scanplan::*;

use flo_render_software::canvas::*;

#[test]
pub fn render_simple_circle() {
    // Render a basic circle
    let mut drawing = Vec::<Draw>::new();

    drawing.clear_canvas(Color::Rgba(0.0, 0.0, 0.0, 0.0));
    drawing.identity_transform();
    drawing.circle(0.0, 0.0, 0.5);
    drawing.fill_color(Color::Rgba(1.0, 0.0, 0.0, 1.0));
    drawing.fill();

    // Create a canvas drawing and draw the mascot to it
    let mut canvas_drawing = CanvasDrawing::<F32LinearPixel, 4>::empty();
    canvas_drawing.draw(drawing);

    // Time some rendering (useful for profiling/optimisation)
    let mut frame   = vec![0u8; 1920*1080*4];
    let mut rgba    = RgbaFrame::from_bytes(1920, 1080, 2.2, &mut frame).unwrap();

    let renderer = CanvasDrawingRegionRenderer::new(PixelScanPlanner::default(), ScanlineRenderer::new(canvas_drawing.program_runner()), 1080);
    rgba.render(renderer, &canvas_drawing);
}
