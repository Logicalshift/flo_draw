use flo_draw::*;
use flo_canvas::*;

use std::thread;
use std::time::{Instant, Duration};

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

            // Draw a few layers with circles
            for layer_id in [LayerId(0), LayerId(1), LayerId(2)] {
                gc.layer(layer_id);
                gc.clear_layer();

                if layer_id == LayerId(0) {
                    gc.fill_color(Color::Rgba(0.8, 0.0, 0.0, 0.9));
                } else if layer_id == LayerId(1) {
                    gc.fill_color(Color::Rgba(0.2, 0.8, 0.0, 0.9));
                } else if layer_id == LayerId(2) {
                    gc.fill_color(Color::Rgba(0.0, 0.6, 0.8, 0.9));
                }

                for y in 0..30 {
                    let y = y-15;
                    let y = (y as f32) * 100.0;

                    for x in 0..30 {
                        let x = x-15;
                        let x = (x as f32) * 100.0;

                        gc.new_path();
                        gc.circle(x, y, 20.0);
                        gc.fill();
                    }
                }
            }
        });

        // Transform the layers over time without redrawing them
        let start_time = Instant::now();

        loop {
            // Time since the loop started
            let nanos_since_start   = Instant::now().duration_since(start_time).as_nanos();
            let seconds_since_start = (nanos_since_start as f64) / 1_000_000_000.0;
            let frames_since_start  = seconds_since_start / (1.0/60.0);

            canvas.draw(|gc| {
                // Layer 0 moves right
                gc.layer(LayerId(0));
                gc.set_layer_transform(Transform2D::translate((frames_since_start%100.0) as f32, 0.0));

                // Layer 1 moves up at 0.9 speed
                gc.layer(LayerId(1));
                gc.set_layer_transform(Transform2D::translate(0.0, ((frames_since_start*0.9)%100.0) as f32));

                // Layer 2 scales
                gc.layer(LayerId(2));
                gc.set_layer_transform(Transform2D::scale(2.0 + (frames_since_start*0.015).sin() as f32, 2.0 + (frames_since_start*0.015).sin() as f32));
            });

            // Wait for a frame
            thread::sleep(Duration::from_secs_f64(1.0/60.0));
        }
    });
}
