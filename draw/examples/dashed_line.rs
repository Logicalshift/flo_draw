use flo_draw::*;
use flo_draw::canvas::*;

///
/// Draws a dashed line
///
pub fn main() {
    with_2d_graphics(|| {
        // Create a window
        let canvas      = create_canvas_window("Dashed line");

        canvas.draw(|gc| {
            // Set up the canvas
            gc.canvas_height(1000.0);
            gc.center_region(0.0, 0.0, 1000.0, 1000.0);

            gc.line_width(8.0);
            gc.new_dash_pattern();
            gc.dash_length(4.0);
            gc.dash_length(4.0);
            gc.dash_length(2.0);
            gc.dash_length(2.0);
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
        });
    });
}
