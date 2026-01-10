use flo_render_software::draw::*;
use flo_render_software::pixel::*;
use flo_render_software::render::*;
use flo_render_software::scanplan::*;

use flo_render_software::canvas::*;

use std::time::{Instant};

fn draw_thing(drawing: &mut impl GraphicsContext, x: f32, y: f32) {
    drawing.stroke_color(Color::Rgba(0.6, 0.6, 0.6, 1.0));
    drawing.fill_color(Color::Rgba(0.7, 0.7, 0.7, 0.9));
    drawing.line_width(2.0);

    drawing.new_path();
    drawing.circle(x, y, 24.0);
    drawing.fill();
    drawing.stroke();

    /*
    drawing.push_state();
    drawing.sprite_transform(SpriteTransform::Scale(1.2, 1.2));
    drawing.sprite_transform(SpriteTransform::Translate(x, y));
    drawing.draw_sprite(SpriteId(0));
    drawing.pop_state();
    */

    drawing.fill_color(Color::Rgba(0.3, 0.3, 0.3, 1.0));
    drawing.new_path();
    drawing.circle(x, y, 12.0 * 1.2);
    drawing.fill();
}

///
/// Draws some overlapping circles to the terminal
///
pub fn main() {
    // Create a drawing of a triangle
    let mut drawing = vec![];

    drawing.clear_canvas(Color::Rgba(1.0, 0.95, 0.8, 1.0));
    drawing.canvas_height(1080.0);
    drawing.center_region(0.0, 0.0, 1080.0, 1080.0);

    drawing.sprite(SpriteId(0));
    drawing.clear_sprite();
    drawing.fill_color(Color::Rgba(0.3, 0.3, 0.3, 1.0));
    drawing.new_path();
    drawing.circle(0.0, 0.0, 12.0);
    drawing.fill();

    drawing.layer(LayerId(0));

    draw_thing(&mut drawing, 116.2421875, 181.62109375);
    draw_thing(&mut drawing, 139.6796875, 195.40234375);
    draw_thing(&mut drawing, 530.1, 550.2);
    draw_thing(&mut drawing, 550.1, 550.2);
    draw_thing(&mut drawing, 540.0, 540.0);

    // Create a canvas from the drawing
    let mut canvas_drawing = CanvasDrawing::<F32LinearPixel, 4>::empty();
    canvas_drawing.draw(drawing);

    // Time how long it takes to draw the mascot to the canvas (full frames will often involve both of these steps)
    let mut frame   = vec![0u8; 1920*1080*4];
    let mut rgba    = FrameU8Rgba::from_bytes(1920, 1080, 2.2, &mut frame).unwrap();

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
