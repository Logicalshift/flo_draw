use flo_draw::*;
use flo_canvas::*;
use flo_curves::geo::*;

use std::sync::*;

///
/// Displays 'Hello, World' in a window
///
pub fn main() {
    with_2d_graphics(|| {
        let lato        = CanvasFontFace::from_slice(include_bytes!("Lato-Regular.ttf"));

        // Create a window
        let canvas      = create_canvas_window("Hello");

        let hello_size  = lato.measure("Hello, World", 100.0);
        let (min, max)  = hello_size.inner_bounds;

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

            gc.new_path();
            gc.rect(50.0 + min.x() as f32, 500.0 + min.y() as f32, 
                50.0 + max.x() as f32, 500.0 + max.y() as f32);
            gc.stroke();
        });
    });
}
