use flo_render_software::draw::*;
use flo_render_software::pixel::*;
use flo_render_software::render::*;
use flo_render_software::scanplan::*;

use flo_render_software::canvas::*;

use std::time::{Instant};

///
/// Draws FlowBetween's mascot as vector graphics in a window
///
pub fn main() {
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

    let render_start = Instant::now();
    for _ in 0..100 {
        let renderer = CanvasDrawingRegionRenderer::new(PixelScanPlanner::default(), ScanlineRenderer::new(canvas_drawing.program_runner()), 1080);
        rgba.render(renderer, &canvas_drawing);
    }
    let render_time = Instant::now().duration_since(render_start);
    let avg_micros  = render_time.as_micros() / 100;
    println!("Frame render time: {}.{}ms", avg_micros/1000, avg_micros%1000);

    // Render the mascot to the terminal
    let mut term_renderer = TerminalRenderTarget::new(500, 400);

    let renderer = CanvasDrawingRegionRenderer::new(PixelScanPlanner::default(), ScanlineRenderer::new(canvas_drawing.program_runner()), 1080);
    term_renderer.render(renderer, &canvas_drawing);
}
