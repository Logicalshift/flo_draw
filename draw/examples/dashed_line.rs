use flo_draw::*;
use flo_draw::canvas::*;

use std::thread;
use std::time::{Duration};

///
/// Draws a dashed line
///
pub fn main() {
    with_2d_graphics(|| {
        // Create a window
        let canvas      = create_canvas_window("Dashed line");

        let mut offset = 0.0;
        loop {
            canvas.draw(|gc| {
                gc.clear_canvas(Color::Rgba(1.0, 1.0, 1.0, 1.0));

                // Set up the canvas
                gc.canvas_height(1000.0);
                gc.center_region(0.0, 0.0, 1000.0, 1000.0);

                gc.line_width(8.0);
                gc.new_dash_pattern();
                gc.dash_offset(offset % 60.0);
                gc.dash_length(20.0);
                gc.dash_length(20.0);
                gc.dash_length(10.0);
                gc.dash_length(10.0);
                gc.stroke_color(Color::Rgba(0.0, 0.0, 0.6, 1.0));

                gc.new_path();
                gc.rect(100.0, 100.0, 900.0, 900.0);
                gc.stroke();

                gc.new_path();
                gc.move_to(200.0, 200.0);
                gc.line_to(800.0, 800.0);
                gc.stroke();

                gc.new_path();
                gc.circle(300.0, 700.0, 100.0);
                gc.stroke();

                gc.new_path();
                gc.circle(700.0, 300.0, 100.0);
                gc.fill();
            });

            offset += 1.0;
            thread::sleep(Duration::from_nanos(1_000_000_000 / 60));
        }
    });
}
