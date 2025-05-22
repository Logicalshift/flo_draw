use flo_render_software::render::*;
use flo_render_software::canvas::*;

use std::io;

fn draw_scaled_mascot(canvas: &mut impl GraphicsPrimitives, x: f32, y: f32, width: f32, height: f32) {
    canvas.new_path();
    canvas.rect(x, y, x+width, y+height);

    canvas.fill_texture(TextureId(0), x, y+height, x+width, y);
    canvas.fill();
}

///
/// Draws FlowBetween's mascot as a texture at different scales
///
pub fn main() {
    // Load a png file
    let flo_bytes: &[u8] = include_bytes!["flo_drawing_on_window.png"];

    // Create drawing instructions for the png
    let mut canvas = vec![];

    // Clear the canvas and set up the coordinates
    canvas.clear_canvas(Color::Rgba(1.0, 1.0, 1.0, 1.0));
    canvas.canvas_height(1000.0);
    canvas.center_region(0.0, 0.0, 1000.0, 1000.0);

    // Set up the texture
    let (flo_w, flo_h) = canvas.load_texture(TextureId(0), io::Cursor::new(flo_bytes)).unwrap();

    let ratio   = (flo_w as f32)/(flo_h as f32);

    // Draw a bunch of mascots
    draw_scaled_mascot(&mut canvas, 87.5 + 0.0, 0.0, 50.0, 50.0/ratio);
    draw_scaled_mascot(&mut canvas, 87.5 + 75.0, 0.0, 100.0, 100.0/ratio);
    draw_scaled_mascot(&mut canvas, 87.5 + 200.0, 0.0, 200.0, 200.0/ratio);
    draw_scaled_mascot(&mut canvas, 87.5 + 425.0, 0.0, 400.0, 400.0/ratio);
    draw_scaled_mascot(&mut canvas, 250.0, 500.0, 500.0, 500.0/ratio);

    // Render to the terminal window
    render_drawing(&mut TerminalRenderTarget::new(1920, 1080), canvas.iter().cloned());
}
