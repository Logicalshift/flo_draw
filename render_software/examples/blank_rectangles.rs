use flo_render_software::draw::*;
use flo_render_software::pixel::*;
use flo_render_software::render::*;
use flo_render_software::scanplan::*;

use flo_render_software::canvas::*;

///
/// Temp demo that shows a blending problem that emerges from the shard scan planner
///
/// Even though these rectangles are completely transparent, we're getting lines drawn on the resulting canvas, indicating something write with the
/// blending program (or maybe something to do with pre-multiplication?)
///
pub fn main() {
    let mut draw_rectangles = vec![];

    // Create a canvas drawing and draw the mascot to it
    let mut canvas_drawing = CanvasDrawing::<F32LinearPixel, 4>::empty();
    canvas_drawing.draw(vec![Draw::ClearCanvas(Color::Rgba(0.2, 0.8, 0.4, 0.7))]);
    draw_rectangles.canvas_height(1003.0);
    draw_rectangles.center_region(0.0, 0.0, 1000.0, 1000.0);

    // Draw some blank rectangles
    draw_rectangles.fill_color(Color::Rgba(0.0, 0.0, 0.0, 0.0));
    draw_rectangles.new_path();
    draw_rectangles.rect(0.0, 50.0, 400.0, 450.0);
    draw_rectangles.fill();

    draw_rectangles.fill_texture(TextureId(0), 600.0, 400.0, 1000.0, 800.0);
    draw_rectangles.new_path();
    draw_rectangles.rect(600.0, 400.0, 1000.0, 800.0);
    draw_rectangles.fill();

    draw_rectangles.fill_texture(TextureId(1), 600.0, 50.0, 1000.0, 450.0);
    draw_rectangles.new_path();
    draw_rectangles.rect(600.0, 50.0, 1000.0, 450.0);
    draw_rectangles.fill();

    draw_rectangles.fill_texture(TextureId(1), 0.0, 400.0, 400.0, 800.0);
    draw_rectangles.new_path();
    draw_rectangles.rect(0.0, 400.0, 400.0, 800.0);
    draw_rectangles.fill();

    canvas_drawing.draw(draw_rectangles.iter().cloned());

    // Render the mascot to the terminal
    let mut term_renderer = TerminalRenderTarget::new(1920, 1080);

    let renderer = CanvasDrawingRegionRenderer::new(ShardScanPlanner::default(), ScanlineRenderer::new(canvas_drawing.program_runner(1080.0)), 1080);
    term_renderer.render(renderer, &canvas_drawing);
}

