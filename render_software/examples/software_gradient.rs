use flo_render_software::render::*;
use flo_render_software::canvas::*;

use std::f32;

///
/// Draws a simple linear gradient
///
pub fn main() {
    // Create drawing instructions for the png
    let mut canvas = vec![];

    let angle = (30.0/360.0) * (2.0 * f32::consts::PI);

    // Clear the canvas and set up the coordinates
    canvas.clear_canvas(Color::Rgba(1.0, 1.0, 1.0, 1.0));
    canvas.canvas_height(1000.0);
    canvas.center_region(0.0, 0.0, 1000.0, 1000.0);

    canvas.layer(LayerId(0));
    canvas.clear_layer();

    // Set up the canvas
    canvas.canvas_height(1000.0);
    canvas.center_region(0.0, 0.0, 1000.0, 1000.0);

    // Set up a gradient
    canvas.create_gradient(GradientId(1), Color::Rgba(0.8, 0.0, 0.0, 1.0));
    canvas.gradient_stop(GradientId(1), 0.33, Color::Rgba(0.3, 0.8, 0.0, 1.0));
    canvas.gradient_stop(GradientId(1), 0.66, Color::Rgba(0.0, 0.3, 0.8, 1.0));
    canvas.gradient_stop(GradientId(1), 1.0, Color::Rgba(0.6, 0.3, 0.9, 1.0));

    let x1 = 500.0 - 300.0*f32::cos(angle);
    let y1 = 500.0 - 300.0*f32::sin(angle);
    let x2 = 500.0 + 300.0*f32::cos(angle);
    let y2 = 500.0 + 300.0*f32::sin(angle);

    // Draw a circle using the gradient
    canvas.new_path();
    canvas.circle(500.0, 500.0, 250.0);
    canvas.fill_gradient(GradientId(1), x1, y1, x2, y2);
    canvas.fill();

    canvas.line_width(4.0);
    canvas.stroke_color(Color::Rgba(0.0, 0.0, 0.0, 1.0));
    canvas.stroke();

    // Draw indicators where the gradient is moving between
    canvas.line_width(1.0);

    canvas.new_path();
    canvas.circle(x1, y1, 8.0);
    canvas.stroke();

    canvas.new_path();
    canvas.circle(x2, y2, 8.0);
    canvas.stroke();

    // Render to the terminal window
    render_drawing(&mut TerminalRenderTarget::new(1920, 1080), canvas.iter().cloned());
}
