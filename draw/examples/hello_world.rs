use flo_draw::*;
use flo_canvas::*;

use std::sync::*;

///
/// Displays 'Hello, World' in a window
///
pub fn main() {
    with_2d_graphics(|| {
        let lato    = CanvasFontFace::from_slice(&include_bytes!("Lato-Regular.ttf").clone());

        // Create a window
        let canvas  = create_canvas_window("Hello");

        // Say 'hello, world'
        canvas.draw(|gc| {
            // Set up the canvas
            gc.canvas_height(1000.0);
            gc.center_region(0.0, 0.0, 1000.0, 1000.0);

            // Load a font
            gc.define_font_data(FontId(1), Arc::clone(&lato));
            gc.set_font_size(FontId(1), 100.0);

            // Draw some text in our font
            gc.fill_color(Color::Rgba(0.0, 0.0, 0.6, 1.0));
            gc.draw_text(FontId(1), "Hello, World".to_string(), 50.0, 500.0);
        });
    });
}
