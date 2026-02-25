use flo_draw::*;
use flo_canvas::*;

///
/// Displays 'Hello, World' in a window
///
pub fn main() {
    with_2d_graphics(|| {
        // Create a window
        let canvas = create_drawing_window("Layer transforms");

        canvas.draw(|gc| {
            // Set up the canvas
            gc.canvas_height(1000.0);
            gc.center_region(0.0, 0.0, 1000.0, 1000.0);
        });
    });
}
