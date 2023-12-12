use flo_render_software::draw::*;
use flo_render_software::pixel::*;
use flo_render_software::render::*;
use flo_render_software::scanplan::*;

use flo_render_software::canvas::*;

use std::time::{Instant};

///
/// Draws a triangle to the terminal
///
pub fn main() {
    // Create a drawing of a triangle
    let mut drawing = vec![];

    drawing.clear_canvas(Color::Rgba(1.0, 0.95, 0.8, 1.0));
    drawing.canvas_height(1080.0);
    drawing.center_region(0.0, 0.0, 1080.0, 1080.0);

    drawing.new_path();
    drawing.move_to(400.0, 100.0);
    drawing.line_to(540.0, 800.0);
    drawing.line_to(680.0, 100.0);
    drawing.fill_color(Color::Rgba(0.3, 0.8, 0.0, 1.0));
    drawing.fill();

    // Create a canvas from the drawing
    let mut canvas_drawing = CanvasDrawing::<F32LinearPixel, 4>::empty();
    canvas_drawing.draw(drawing);

    // Time how long it takes to draw the mascot to the canvas (full frames will often involve both of these steps)
    let mut frame   = vec![0u8; 1920*1080*4];
    let mut rgba    = RgbaFrame::from_bytes(1920, 1080, 2.2, &mut frame).unwrap();

    for _ in 0..10 {
        let renderer = CanvasDrawingRegionRenderer::new(ShardScanPlanner::default(), ScanlineRenderer::new(canvas_drawing.program_runner(1080.0)), 1080);
        rgba.render(renderer, &canvas_drawing);
    }

    let render_start = Instant::now();
    for _ in 0..100 {
        let renderer = CanvasDrawingRegionRenderer::new(ShardScanPlanner::default(), ScanlineRenderer::new(canvas_drawing.program_runner(1080.0)), 1080);
        rgba.render(renderer, &canvas_drawing);
    }
    let render_time = Instant::now().duration_since(render_start);
    let avg_micros  = render_time.as_micros() / 100;
    println!("F32 frame render time: {}.{}ms", avg_micros/1000, avg_micros%1000);

    // Render the drawing to the terminal
    let mut term_renderer = TerminalRenderTarget::new(1920, 1080);

    let renderer = CanvasDrawingRegionRenderer::new(ShardScanPlanner::default(), ScanlineRenderer::new(canvas_drawing.program_runner(1080.0)), 1080);
    term_renderer.render(renderer, &canvas_drawing);
}
