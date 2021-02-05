use flo_draw::*;
use flo_canvas::*;
use flo_binding::*;

use std::thread;
use std::time::{Duration};

///
/// Simple example that displays a canvas window, then updates the title once a second
///
pub fn main() {
    // 'with_2d_graphics' is used to support operating systems that can't run event loops anywhere other than the main thread
    with_2d_graphics(|| {
        // Create some window properties with a title binding
        let title                   = bind("Title".to_string());
        let mut window_properties   = WindowProperties::from(&());

        window_properties.title     = BindRef::from(title.clone());

        // Create a window with these properties
        let canvas = create_canvas_window(window_properties);

        // Render a triangle to it
        canvas.draw(|gc| {
            // Clear the canvas and set up the coordinates
            gc.clear_canvas(Color::Rgba(0.3, 0.2, 0.0, 1.0));
            gc.canvas_height(1000.0);
            gc.center_region(0.0, 0.0, 1000.0, 1000.0);

            // Draw a rectangle...
            gc.new_path();
            gc.move_to(0.0, 0.0);
            gc.line_to(1000.0, 0.0);
            gc.line_to(1000.0, 1000.0);
            gc.line_to(0.0, 1000.0);
            gc.line_to(0.0, 0.0);

            gc.fill_color(Color::Rgba(1.0, 1.0, 0.8, 1.0));
            gc.fill();

            // Draw a triangle on top
            gc.new_path();
            gc.move_to(200.0, 200.0);
            gc.line_to(800.0, 200.0);
            gc.line_to(500.0, 800.0);
            gc.line_to(200.0, 200.0);

            gc.fill_color(Color::Rgba(0.0, 0.0, 0.8, 1.0));
            gc.fill();
        });

        // Fairly boring 'update the title once a second' sequence
        let mut count = 0;
        loop {
            thread::sleep(Duration::from_secs(1));

            count += 1;
            title.set(format!("Running for {} seconds", count));
        }
    });
}
