use flo_draw::*;
use flo_draw::canvas::*;

///
/// Demonstrates using a clipping path
///
pub fn main() {
    with_2d_graphics(|| {
        // Create a window
        let canvas      = create_canvas_window("Clipping demonstration");

        // Say 'hello, world'
        canvas.draw(|gc| {
            // Set up the canvas
            gc.clear_canvas(Color::Rgba(0.2, 0.4, 0.0, 1.0));
            gc.canvas_height(1000.0);
            gc.center_region(0.0, 0.0, 1000.0, 1000.0);

            gc.new_path();
            gc.circle(500.0, 500.0, 200.0);
            gc.clip();

            gc.new_path();
            gc.rect(0.0, 0.0, 1000.0, 1000.0);
            gc.fill_color(Color::Rgba(0.0, 0.0, 0.6, 1.0));
            gc.fill();
        });
    });
}
