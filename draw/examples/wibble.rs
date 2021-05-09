use flo_draw::*;
use flo_draw::canvas::*;
use flo_curves::*;
use flo_curves::bezier::*;
use flo_curves::bezier::path::*;

use futures::prelude::*;
use futures::executor;
use futures::stream;

use std::f64;
use std::thread;
use std::sync::*;
use std::time::{Duration, Instant};

///
/// Demonstrates capturing the paths for text rendering, and then distorting them using flo_curves
///
pub fn main() {
    with_2d_graphics(|| {
        let lato        = CanvasFontFace::from_slice(include_bytes!("Lato-Regular.ttf"));

        // Create a window
        let canvas      = create_drawing_window("Wibble");

        // Measure the text
        let text_string = "Wibble";
        let wibble_size = measure_text(&lato, text_string, 200.0);
        let (min, max)  = wibble_size.inner_bounds;

        let x_pos       = (1000.0 - (max.x()-min.x()))/2.0;
        let y_pos       = 400.0;

        // Render the text to a set of paths
        let mut render_text = vec![];
        render_text.define_font_data(FontId(1), Arc::clone(&lato));
        render_text.set_font_size(FontId(1), 200.0);
        render_text.draw_text(FontId(1), text_string.to_string(), x_pos as _, y_pos as _);

        // Lay out the text, convert the glyphs to paths, convert the drawing instructions to SimpleBezierPaths
        let render_text     = stream::iter(render_text.into_iter());
        let text_paths      = drawing_with_laid_out_text(render_text);
        let text_paths      = drawing_with_text_as_paths(text_paths);
        let text_paths      = drawing_to_paths::<SimpleBezierPath, _>(text_paths);
        let text_paths      = executor::block_on(async move { text_paths.collect::<Vec<_>>().await });

        // Draw the text with a moving distortion
        let start_time = Instant::now();

        loop {
            // Get the current time where we're rendering this
            let since_start             = Instant::now().duration_since(start_time);
            let since_start             = since_start.as_nanos() as f64;
            let amplitude               = 12.0;

            // Distort each of the paths in turn
            let distorted_text_paths    = text_paths.iter()
                .map(|path_set| path_set.iter()
                    .map(move |path: &SimpleBezierPath| distort_path::<_, _, SimpleBezierPath>(path, |point: Coord2, _curve, _t| {
                        let distance    = point.magnitude();
                        let ripple      = (since_start / (f64::consts::PI * 500_000_000.0)) * 10.0;

                        let offset_x    = (distance / (f64::consts::PI*5.0) + ripple).sin() * amplitude * 0.5;
                        let offset_y    = (distance / (f64::consts::PI*4.0) + ripple).cos() * amplitude * 0.5;

                        Coord2(point.x() + offset_x, point.y() + offset_y)
                    }, 1.0, 0.1).unwrap())
                    .collect::<Vec<_>>());

            // Render the current frame
            canvas.draw(|gc| {
                // Clear the canvas
                gc.clear_canvas(Color::Rgba(0.7, 0.9, 0.9, 1.0));
                gc.canvas_height(1000.0);
                gc.center_region(0.0, 0.0, 1000.0, 1000.0);

                gc.fill_color(Color::Rgba(1.0, 1.0, 1.0, 1.0));
                gc.stroke_color(Color::Rgba(0.0, 0.0, 0.0, 1.0));
                gc.line_width(6.0);

                // Render the distorted paths
                for path_set in distorted_text_paths {
                    gc.new_path();

                    for path in path_set {
                        gc.bezier_path(&path);
                        gc.close_path();
                    }

                    gc.fill();
                    gc.stroke();
                }

            });

            // Wait for the next frame
            thread::sleep(Duration::from_nanos(1_000_000_000 / 60));
        }
    });
}
