use flo_draw::*;
use flo_draw::canvas::*;

use std::thread;
use std::time::{Duration};

///
/// Dynamic textures can be used to have a texture created from a sprite that is re-rendered to match the resolution of the canvas
///
pub fn main() {
    // 'with_2d_graphics' is used to support operating systems that can't run event loops anywhere other than the main thread
    with_2d_graphics(|| {
        // Create a window
        let canvas = create_drawing_window("Dynamic texture rendered from a sprite");

        // Create a texture by drawing to a sprite
        canvas.draw(|gc| {
            // Clear the canvas and set up the coordinates
            gc.clear_canvas(Color::Rgba(1.0, 1.0, 1.0, 1.0));
            gc.canvas_height(1000.0);
            gc.center_region(0.0, 0.0, 1000.0, 1000.0);
            
            // Set up the sprite
            gc.sprite(SpriteId(0));
            gc.clear_sprite();

            gc.new_path();
            gc.rect(0.0, 0.0, 100.0, 100.0);
            gc.fill_color(Color::Rgba(0.3, 0.6, 1.0, 1.0));
            gc.fill();

            gc.new_path();
            gc.circle(50.0, 50.0, 40.0);
            gc.fill_color(Color::Rgba(0.3, 1.0, 0.6, 1.0));
            gc.fill();

            gc.layer(LayerId(0));

            // Create a dynamic texture that fits on a 100x100 block on the screen
            gc.create_dynamic_texture(TextureId(0), SpriteId(0), 0.0, 0.0, 100.0, 100.0, 100.0, 100.0);
        });

        let mut angle = 0.0;

        loop {
            // Render the texture to the window
            canvas.draw(|gc| {
                gc.layer(LayerId(0));
                gc.clear_layer();

                // Draw the texture with 4x the size so the pixels can be seen (also spin it)
                gc.new_path();
                gc.rect(0.0, 0.0, 1000.0, 1000.0);
                gc.fill_texture(TextureId(0), 200.0, 200.0, 800.0, 800.0);

                gc.fill_transform(Transform2D::translate(-500.0, -500.0));
                gc.fill_transform(Transform2D::rotate_degrees(angle));
                gc.fill_transform(Transform2D::translate(500.0, 500.0));

                gc.fill();

                // Draw at 1x scale
                gc.new_path();
                gc.rect(100.0, 100.0, 200.0, 200.0);
                gc.fill_texture(TextureId(0), 100.0, 100.0, 200.0, 200.0);
                gc.fill();
            });

            // Wait for the next frame
            thread::sleep(Duration::from_nanos(1_000_000_000 / 60));

            // Rotate the texture
            angle += 0.1;
        }
    });
}
