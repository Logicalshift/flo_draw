use flo_draw::*;
use flo_canvas::*;

use futures::prelude::*;
use futures::executor;

///
/// Simple example that displays a canvas window and renders a triangle
///
pub fn main() {
    // 'with_2d_graphics' is used to support operating systems that can't run event loops anywhere other than the main thread
    with_2d_graphics(|| {
        // Create a render window and loop until it stops sending events
        executor::block_on(async {
            // Create a window
            let (canvas, mut events) = create_canvas_window();

            // Render a triangle to it
            canvas.draw(|gc| {
                gc.clear_canvas();
                gc.canvas_height(1000.0);
                gc.center_region(0.0, 0.0, 1000.0, 1000.0);

                gc.new_path();
                gc.move_to(200.0, 200.0);
                gc.line_to(800.0, 200.0);
                gc.line_to(500.0, 800.0);
                gc.line_to(200.0, 200.0);

                gc.fill_color(Color::Rgba(0.0, 0.0, 0.8, 1.0));
                gc.fill();
            });

            // Wait until it stops producing events
            while let Some(_evt) = events.next().await {
            }
        });
    });
}
