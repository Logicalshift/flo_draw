use flo_draw::*;
use flo_draw::canvas::*;

use futures::prelude::*;

use std::thread;
use std::time::{Duration};

///
/// Draws to two windows simultaneously
///
pub fn main() {
    with_2d_graphics(|| {
        // Create a canvas window: canvases store their drawing instructions and can mirror them to multiple targets
        let canvas = create_canvas_window("Mirror windows");

        // Create a duplicate window from the same canvas (gluting creates windows on top of each other, annoyingly)
        let _ = create_drawing_window_from_stream(canvas.stream().ready_chunks(10000), "Second window (might need to drag to see the other window)");

        // Draw to both windows at once
        let mut p = 0.0f32;
        loop {
            p += 0.02;

            canvas.draw(|gc| {
                gc.clear_canvas(Color::Rgba(1.0, 1.0, 1.0, 1.0));

                gc.canvas_height(1000.0);

                let x = p.sin() * 500.0;
                let y = (p*3.0).cos() * 200.0;

                gc.new_path();
                gc.circle(x, y, 50.0);
                gc.fill_color(Color::Rgba(0.7, 0.0, 0.0, 1.0));
                gc.fill();
            });

            // 60fps
            thread::sleep(Duration::from_nanos(1_000_000_000 / 60));
        }
    });
}
