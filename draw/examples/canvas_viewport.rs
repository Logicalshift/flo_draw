//!
//! Demonstrates using the window properties to change the viewport onto the canvas
//!
//! Normally the viewport uses an algorithm that fits the canvas to the canvas height
//! specified by the rendering instructions. However, this requires redrawing the canvas
//! if the ratio of the window or the scaling changes, which is somewhat wasteful and
//! can be complicated. `flo_draw` does store the existing rendering, so using a 
//! canvas viewport can avoid the need to redraw the window when its size changes.
//!
//! It's also an easier way to request different ways of fitting the rendering into
//! the window.
//!

use flo_draw::*;
use flo_draw::canvas::*;
use flo_binding::*;

use futures::prelude::*;
use futures::executor;

use std::sync::*;

pub fn main() {
    with_2d_graphics(|| {
        let lato = CanvasFontFace::from_slice(include_bytes!("Lato-Regular.ttf"));

        // Create the window properties and add in a binding for the viewport
        let mut window_properties   = WindowProperties::from(&"Canvas viewport");
        let viewport_bounds         = bind(ViewportBounds::Width(1024.0));

        window_properties.viewport_bounds = viewport_bounds.clone().into();

        // Create a window with these properties
        let (canvas, events) = create_drawing_window_with_events(window_properties);

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
            gc.layout_text(FontId(1), "ViewportBounds::Width()".to_string());

            gc.draw_text_layout();
        });

        // Run an event loop to change the layout whenever the user clicks the mouse
        executor::block_on(async move {
            let mut events = events;

            // Bindings we cycle through
            let bindings            = [ViewportBounds::Width(1024.0), ViewportBounds::All, ViewportBounds::CenterRegion((100.0, 100.0), (924.0, 668.0)), ViewportBounds::FitExact((0.0, 0.0), (1024.0, 768.0))];
            let binding_text        = ["ViewportBounds::Width()", "ViewportBounds::All", "ViewportBounds::CenterRegion()", "ViewportBounds::FitExact()"];
            let mut current_binding = 0;

            // Monitor for events
            while let Some(evt) = events.next().await {
                match evt {
                    DrawEvent::Pointer(PointerAction::ButtonUp, _, _) => {
                        // User has clicked the mouse: update the binding we're using
                        current_binding = (current_binding + 1) % bindings.len();
                        viewport_bounds.set(bindings[current_binding]);

                        canvas.draw(|gc| {
                            gc.layer(LayerId(2));
                            gc.clear_layer();

                            gc.set_font_size(FontId(1), 48.0);
                            gc.fill_color(Color::Rgba(0.0, 0.0, 0.0, 1.0));

                            gc.begin_line_layout(512.0, 128.0, TextAlignment::Center);
                            gc.layout_text(FontId(1), binding_text[current_binding].to_string());

                            gc.draw_text_layout();
                        });
                    }

                    _ => { }
                }
            }
        })
    });
}