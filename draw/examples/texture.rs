use flo_draw::*;
use flo_draw::canvas::*;

use std::io;

///
/// Simple example that displays a canvas window and renders a triangle
///
pub fn main() {
    // 'with_2d_graphics' is used to support operating systems that can't run event loops anywhere other than the main thread
    with_2d_graphics(|| {
        // Load a png file
        let flo_bytes: &[u8] = include_bytes!["flo_and_carrot.png"];

        // Create a window
        let canvas = create_drawing_window("Flo with carrot");

        // Render a triangle to it
        canvas.draw(|gc| {
            // Clear the canvas and set up the coordinates
            gc.clear_canvas(Color::Rgba(1.0, 1.0, 1.0, 1.0));
            gc.canvas_height(1000.0);
            gc.center_region(0.0, 0.0, 1000.0, 1000.0);

            // Set up the texture
            let (flo_w, flo_h) = gc.load_texture(TextureId(0), io::Cursor::new(flo_bytes)).unwrap();

            let ratio   = (flo_w as f32)/(flo_h as f32);
            let height  = 1000.0 / ratio;
            let y_pos   = (1000.0-height)/2.0;

            // Draw a rectangle...
            gc.new_path();
            gc.rect(0.0, y_pos, 1000.0, y_pos+height);

            // Fill with the texture we just loaded
            gc.fill_texture(TextureId(0), 0.0, y_pos+flo_h as f32, flo_w as _, y_pos);
            gc.fill();
        });
    });
}
