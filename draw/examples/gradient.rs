use flo_draw::*;
use flo_canvas::*;

use std::thread;
use std::time::{Duration};

///
/// Displays a circle with a linear gradient fill
///
pub fn main() {
    with_2d_graphics(|| {
        // Create a window
        let canvas      = create_drawing_window("Gradient");

        let mut angle   = 0.0;

        loop {
            // Draw a circle
            canvas.draw(|gc| {
                // Set up the canvas
                gc.canvas_height(1000.0);
                gc.center_region(0.0, 0.0, 1000.0, 1000.0);

                // Set up a gradient
                gc.new_gradient(GradientId(1), Color::Rgba(0.8, 0.0, 0.0, 1.0));
                gc.gradient_stop(GradientId(1), 0.33, Color::Rgba(0.3, 0.8, 0.0, 1.0));
                gc.gradient_stop(GradientId(1), 0.66, Color::Rgba(0.0, 0.3, 0.8, 1.0));
                gc.gradient_stop(GradientId(1), 1.0, Color::Rgba(0.6, 0.3, 0.9, 1.0));

                // Draw a circle using the gradient
                gc.new_path();
                gc.circle(500.0, 500.0, 250.0);
                //gc.fill_gradient(GradientId(1), 250.0, 250.0, 250.0, 750.0);
                gc.fill_gradient(GradientId(1), 500.0 - 250.0*f32::cos(angle), 500.0 - 250.0*f32::sin(angle), 500.0 + 250.0*f32::cos(angle), 500.0 + 250.0*f32::sin(angle));
                gc.fill();

                gc.line_width(4.0);
                gc.stroke_color(Color::Rgba(0.0, 0.0, 0.0, 1.0));
                gc.stroke();
            });

            // Wait a frame
            thread::sleep(Duration::from_nanos(1_000_000_000 / 60));
            angle += 0.01;
        }
    });
}
