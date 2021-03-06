use flo_draw::*;
use flo_canvas::*;

use std::sync::*;

///
/// Erases 'Hello, World' from a rectangle to allow the background to show through
///
pub fn main() {
    with_2d_graphics(|| {
        let lato        = CanvasFontFace::from_slice(include_bytes!("Lato-Regular.ttf"));

        // Create a window
        let canvas      = create_canvas_window("Hello");

        let hello_size  = measure_text(&lato, "Hello, World", 100.0);
        let (min, max)  = hello_size.inner_bounds;

        let x_pos       = (1000.0 - (max.x()-min.x()))/2.0;
        let y_pos       = (1000.0 - (max.y()-min.y()))/2.0;

        // Say 'hello, world'
        canvas.draw(|gc| {
            // Set up the canvas
            gc.canvas_height(1000.0);
            gc.center_region(0.0, 0.0, 1000.0, 1000.0);

            gc.layer(LayerId(2));

            // Draw a rectangle
            gc.new_path();
            gc.rect(100.0, 100.0, 900.0, 900.0);
            gc.fill_color(Color::Rgba(0.0, 0.0, 0.6, 1.0));
            gc.fill();

            // Load a font
            gc.define_font_data(FontId(1), Arc::clone(&lato));
            gc.set_font_size(FontId(1), 100.0);

            // Erase a hole in our text
            gc.blend_mode(BlendMode::DestinationOut);
            gc.fill_color(Color::Rgba(0.0, 0.0, 0.0, 1.0));
            gc.draw_text(FontId(1), "Hello, World".to_string(), x_pos as _, y_pos as _);
            gc.blend_mode(BlendMode::SourceOver);

            // Draw a line underneath (it will show through the erased section)
            gc.layer(LayerId(1));
            gc.new_path();
            gc.rect(300.0, 450.0, 700.0, 490.0);
            gc.fill_color(Color::Rgba(0.6, 0.0, 0.0, 1.0));
            gc.fill();
        });
    });
}
