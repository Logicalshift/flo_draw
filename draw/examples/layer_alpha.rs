use flo_draw::*;
use flo_canvas::*;

use std::thread;
use std::sync::*;
use std::time::{Duration};

///
/// Displays 'Hello, World' in a window
///
pub fn main() {
    with_2d_graphics(|| {
        let lato        = CanvasFontFace::from_slice(include_bytes!("Lato-Regular.ttf"));

        // Create a window
        let canvas      = create_drawing_window("Hello");

        let hello_size  = measure_text(&lato, "Hello", 100.0);

        let (min, max)  = hello_size.inner_bounds;
        let hello_x_pos = (1000.0 - (max.x()-min.x()))/2.0;
        let hello_y_pos = (1000.0 - (max.y()-min.y()))/2.0;

        let world_size  = measure_text(&lato, "World", 100.0);

        let (min, max)  = world_size.inner_bounds;
        let world_x_pos = (1000.0 - (max.x()-min.x()))/2.0;
        let world_y_pos = (1000.0 - (max.y()-min.y()))/2.0;

        // Say 'hello, world' by cross-fading layers
        canvas.draw(|gc| {
            // Set up the canvas
            gc.canvas_height(1000.0);
            gc.center_region(0.0, 0.0, 1000.0, 1000.0);

            // Load a font
            gc.define_font_data(FontId(1), Arc::clone(&lato));
            gc.set_font_size(FontId(1), 100.0);

            // First layer: 'hello'
            gc.layer(LayerId(1));
            gc.layer_alpha(LayerId(1), 1.0);
            gc.fill_color(Color::Rgba(0.0, 0.0, 0.6, 1.0));
            gc.draw_text(FontId(1), "Hello".to_string(), hello_x_pos as _, hello_y_pos as _);

            // Second layer: 'world'
            gc.layer(LayerId(2));
            gc.layer_alpha(LayerId(2), 0.0);
            gc.fill_color(Color::Rgba(0.0, 0.2, 0.5, 1.0));
            gc.draw_text(FontId(1), "World".to_string(), world_x_pos as _, world_y_pos as _);
        });

        // Update the canvas every 1/60th of a second
        let mut blend: f64 = 0.0;
        loop {
            // Update the layer alpha blending
            blend       += 1.0 / 60.0;

            let blend1  = (blend.cos() + 1.0) / 2.0;
            let blend2  = 1.0 - blend1;

            canvas.draw(|gc| {
                gc.layer_alpha(LayerId(1), blend2);
                gc.layer_alpha(LayerId(2), blend1);
            });

            thread::sleep(Duration::from_nanos(1_000_000_000 / 60));
        }
    });
}
