use flo_render_software::render::*;
use flo_render_software::canvas::*;

///
/// Simple example that displays a canvas window and renders a triangle
///
pub fn main() {
    let mut draw = vec![];

    // Clear the canvas and set up the coordinates
    draw.clear_canvas(Color::Rgba(0.0, 1.0, 0.0, 1.0));
    draw.canvas_height(1000.0);
    draw.center_region(0.0, 0.0, 1000.0, 1000.0);

    // Create a triangle sprite
    draw.sprite(SpriteId(0));
    draw.clear_sprite();
    draw.new_path();
    draw.move_to(200.0, 200.0);
    draw.line_to(800.0, 200.0);
    draw.line_to(500.0, 800.0);
    draw.line_to(200.0, 200.0);

    draw.fill_color(Color::Rgba(0.8, 0.4, 0.2, 1.0));
    draw.fill();

    // Draw the triangle in a few places
    draw.layer(LayerId(0));

    /*
    draw.sprite_transform(SpriteTransform::Identity);
    draw.draw_sprite(SpriteId(0));

    draw.sprite_transform(SpriteTransform::Identity);
    draw.sprite_transform(SpriteTransform::Translate(100.0, 100.0));
    draw.draw_sprite(SpriteId(0));

    draw.sprite_transform(SpriteTransform::Identity);
    draw.sprite_transform(SpriteTransform::Translate(200.0, 100.0));
    draw.draw_sprite(SpriteId(0));

    draw.sprite_transform(SpriteTransform::Identity);
    draw.sprite_transform(SpriteTransform::Translate(300.0, 100.0));
    draw.draw_sprite(SpriteId(0));
    */

    // Render to the terminal window
    render_drawing(&mut TerminalRenderTarget::new(1920, 1080), draw.iter().cloned());
}
