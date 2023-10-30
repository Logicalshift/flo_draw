use flo_draw::*;
use flo_canvas::*;

///
/// Simple example that displays a canvas window and renders a triangle
///
pub fn main() {
    with_2d_graphics(|| {
        // Create a window
        let canvas = create_drawing_window("Basic sprite rendering");

        // Sprites are a way to rapidly repeat a set of drawing instructions
        canvas.draw(|gc| {
            // Clear the canvas and set up the coordinates
            gc.clear_canvas(Color::Rgba(0.0, 1.0, 0.0, 1.0));
            gc.canvas_height(1000.0);
            gc.center_region(0.0, 0.0, 1000.0, 1000.0);

            // Create a triangle sprite
            gc.sprite(SpriteId(0));
            gc.clear_sprite();
            gc.new_path();
            gc.move_to(200.0, 200.0);
            gc.line_to(800.0, 200.0);
            gc.line_to(500.0, 800.0);
            gc.line_to(200.0, 200.0);

            gc.fill_color(Color::Rgba(0.8, 0.4, 0.2, 1.0));
            gc.fill();

            // Draw the triangle in a few places
            gc.layer(LayerId(0));

            gc.sprite_transform(SpriteTransform::Identity);
            gc.draw_sprite(SpriteId(0));

            gc.sprite_transform(SpriteTransform::Identity);
            gc.sprite_transform(SpriteTransform::Translate(100.0, 100.0));
            gc.draw_sprite(SpriteId(0));

            gc.sprite_transform(SpriteTransform::Identity);
            gc.sprite_transform(SpriteTransform::Translate(200.0, 100.0));
            gc.draw_sprite(SpriteId(0));

            gc.sprite_transform(SpriteTransform::Identity);
            gc.sprite_transform(SpriteTransform::Translate(300.0, 100.0));
            gc.draw_sprite(SpriteId(0));
        });
    });
}
