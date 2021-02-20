use flo_draw::*;
use flo_canvas::*;

use futures::prelude::*;
use futures::executor;

///
/// Demonstrates how to follow the mouse cursor around by tracking events
///
/// `create_canvas_window_with_events` works like `create_canvas_window` except it will also return a stream of events
/// for that window. We can track these by monitoring the stream in a futures block created with `executor::block_on`.
///
/// In this case, we watch for pointer events and render a circle to track where the mouse is on a new layer. This 
/// also demonstrates that layers can be used to partially update the canvas without erasing whatever else is drawn,
/// and shows that we can get canvas coordinates directly from a mouse event.
///
/// Is supposed to stop once the window is closed, but glutin appears to not always respond correctly to setting the
/// control flow to ControlFlow::Exit.
///
pub fn main() {
    // 'with_2d_graphics' is used to support operating systems that can't run event loops anywhere other than the main thread
    with_2d_graphics(|| {
        // Create a window and an event queue
        let (canvas, events) = create_canvas_window_with_events("Mouse tracking");

        // Render the window background on layer 0 (just a triangle)
        canvas.draw(|gc| {
            // Clear the canvas and set up the coordinates
            gc.clear_canvas(Color::Rgba(0.3, 0.2, 0.0, 1.0));
            gc.canvas_height(1000.0);
            gc.center_region(0.0, 0.0, 1000.0, 1000.0);

            // We'll draw some graphics to layer 0 (we can leave these alone as we track the mouse around)
            gc.layer(LayerId(0));

            // Draw a rectangle...
            gc.new_path();
            gc.move_to(0.0, 0.0);
            gc.line_to(1000.0, 0.0);
            gc.line_to(1000.0, 1000.0);
            gc.line_to(0.0, 1000.0);
            gc.line_to(0.0, 0.0);

            gc.fill_color(Color::Rgba(1.0, 1.0, 0.8, 1.0));
            gc.fill();

            // Draw a triangle on top
            gc.new_path();
            gc.move_to(200.0, 200.0);
            gc.line_to(800.0, 200.0);
            gc.line_to(500.0, 800.0);
            gc.line_to(200.0, 200.0);

            gc.fill_color(Color::Rgba(0.0, 0.0, 0.8, 1.0));
            gc.fill();
        });

        // Track mouse events and render a circle centered on the current position (we use layer 1 for this so we don't have to re-render the whole canvas)
        executor::block_on(async move {
            let mut events = events;

            // Main event loop
            while let Some(event) = events.next().await {
                match event {
                    // Track any event relating to the pointer
                    DrawEvent::Pointer(_action, _id, state) => {
                        if let Some((x, y)) = &state.location_in_canvas {
                            // Draw a circle at the mouse position
                            canvas.draw(|gc| {
                                // Draw on layer 1 to avoid disrupting the image underneath
                                gc.layer(LayerId(1));
                                gc.clear_layer();

                                gc.new_path();
                                gc.circle(*x as _, *y as _, 20.0);

                                gc.stroke_color(Color::Rgba(0.1, 0.1, 0.1, 0.8));
                                gc.line_width_pixels(3.0);
                                gc.stroke();

                                gc.stroke_color(Color::Rgba(0.6, 0.9, 0.6, 0.8));
                                gc.line_width_pixels(2.0);
                                gc.stroke();
                            });
                        }
                    }

                    // Ignore other events
                    _ => {}
                }
            }
        })
    });
}
