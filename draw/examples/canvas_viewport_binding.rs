//!
//! Demonstrates using a computed viewport binding to resize the canvas dynamically.
//! In this case, we're using it to make the canvas stay at a fixed size instead of
//! scaling to fit the window.
//!

use flo_draw::*;
use flo_draw::canvas::*;
use flo_binding::*;

use std::sync::*;

pub fn main() {
    with_2d_graphics(|| {
        let lato = CanvasFontFace::from_slice(include_bytes!("Lato-Regular.ttf"));

        // Create the window properties with a computed binding that maps directly to the window size
        let window_properties   = WindowProperties::from(&"Canvas viewport");
        let actual_size         = window_properties.actual_size().unwrap();
        let actual_scale        = window_properties.actual_scale().unwrap();
        let window_properties   = window_properties.with_viewport_bounds(
            computed(move || {
                let (w, h) = actual_size.get();
                let scale  = actual_scale.get();

                ViewportBounds::FitExact((0.0, 0.0), (w/scale, h/scale))
            }));

        // Create a window with these properties
        let canvas = create_drawing_window(window_properties);

        // Render a test scene
        canvas.draw(|gc| {
            // Set up a 1024 x 768 drawing area
            gc.clear_canvas(Color::Rgba(0.9, 0.9, 0.9, 1.0));
            gc.canvas_height(768.0);
            gc.center_region(0.0, 0.0, 1024.0, 768.0);

            // Define a font for later
            gc.define_font_data(FontId(1), Arc::clone(&lato));
            gc.set_font_size(FontId(1), 32.0);

            // Draw a grid on layer 0
            gc.layer(LayerId(0));
            gc.line_width(1.0);
            gc.stroke_color(Color::Rgba(0.7, 0.7, 0.7, 1.0));

            gc.new_path();

            let grid_size   = 32.0;
            let grid_offset = 32.0;
            for x in 0..(((1024.0-(grid_offset/2.0))/grid_size) as i32) {
                let pos = (x as f32) * grid_size + grid_offset;

                gc.move_to(pos, grid_offset);
                gc.line_to(pos, 768.0-grid_offset);
            }

            for y in 0..(((768.0-(grid_offset/2.0))/grid_size) as i32) {
                let pos = (y as f32) * grid_size + grid_offset;

                gc.move_to(grid_offset, pos);
                gc.line_to(1024.0-grid_offset, pos);
            }

            gc.stroke();

            // Draw some rectangles and a circle to show the proportions and location of the viewport on layer 1
            gc.layer(LayerId(1));
            gc.clear_layer();

            gc.line_width(1.0);
            gc.stroke_color(Color::Rgba(0.0, 0.0, 0.0, 1.0));

            gc.new_path();
            gc.rect(0.5, 0.5, 1023.5, 767.5);
            gc.stroke();

            gc.new_path();
            gc.rect(0.5, 0.5, 1023.5, 767.5);
            gc.stroke();

            gc.new_path();
            gc.rect(32.5, 32.5, 1023.5-32.0, 767.5-32.0);
            gc.stroke();

            gc.new_path();
            gc.move_to(0.0, 0.0);
            gc.line_to(1024.0, 768.0);
            gc.move_to(1024.0, 0.0);
            gc.line_to(0.0, 768.0);
            gc.stroke();

            gc.new_path();
            gc.circle(512.0, 384.0, 200.0);
            gc.stroke();

            // Layer 2 says what sizing we're using
            gc.layer(LayerId(2));
            gc.clear_layer();

            gc.set_font_size(FontId(1), 48.0);
            gc.fill_color(Color::Rgba(0.0, 0.0, 0.0, 1.0));

            gc.begin_line_layout(512.0, 128.0, TextAlignment::Center);
            gc.layout_text(FontId(1), "Computed viewport".to_string());

            gc.draw_text_layout();

            gc.layer(LayerId(3));
            gc.clear_layer();

            gc.set_font_size(FontId(1), 20.0);
            gc.fill_color(Color::Rgba(0.4, 0.4, 0.4, 1.0));

            gc.set_font_size(FontId(1), 14.0);
            gc.fill_color(Color::Rgba(0.4, 0.4, 0.4, 1.0));

            gc.begin_line_layout(512.0, 68.0, TextAlignment::Center);
            gc.layout_text(FontId(1), "Resize window to see the effects of the viewport bounds setting".to_string());

            gc.draw_text_layout();
        });
    });
}