use flo_draw::*;
use flo_draw::canvas::*;

use rand::*;

use std::io;
use std::thread;
use std::time::{Duration};

struct Ball {
    sprite_id: SpriteId,
    radius:     f64,
    x:          f64,
    y:          f64,

    dx:         f64,
    dy:         f64
}

impl Ball {
    ///
    /// Generates a new ball
    ///
    pub fn random(sprite_id: SpriteId) -> Ball {
        // Decide on how the ball is rendered
        let radius  = 64.0;

        Ball {
            sprite_id:  sprite_id,
            radius:     radius,
            x:          random::<f64>() * 1000.0,
            y:          random::<f64>() * 1000.0 + 64.0,
            dx:         random::<f64>() * 8.0 - 4.0,
            dy:         random::<f64>() * 8.0 - 4.0
        }
    }

    ///
    /// Moves this ball on one frame
    ///
    pub fn update(&mut self) {
        // Collide with the edges of the screen
        if self.x+self.dx+self.radius > 1000.0 && self.dx > 0.0     { self.dx = -self.dx; }
        if self.y+self.dy+self.radius > 1000.0 && self.dy > 0.0     { self.dy = -self.dy; }
        if self.x+self.dx-self.radius < 0.0 && self.dx < 0.0        { self.dx = -self.dx; }
        if self.y+self.dy-self.radius < 0.0 && self.dy < 0.0        { self.dy = -self.dy; }

        // Gravity
        if self.y >= self.radius {
            self.dy -= 0.2;
        }

        // Move this ball in whatever direction it's going
        self.x += self.dx;
        self.y += self.dy;
    }
}

///
/// Bouncing ball example that renders using textured sprites
///
pub fn main() {
    // 'with_2d_graphics' is used to support operating systems that can't run event loops anywhere other than the main thread
    with_2d_graphics(|| {
        // Load a png file
        let flo_bytes: &[u8] = include_bytes!["flo_and_carrot.png"];

        // Create a window with a canvas to draw on
        let canvas = create_canvas_window("Bouncing sprites");

        // Clear the canvas to set a background colour
        let mut flo_w = 0;
        let mut flo_h = 0;
        canvas.draw(|gc| {
            gc.clear_canvas(Color::Rgba(0.6, 0.7, 0.8, 1.0));

            // Set up the texture
            let (w, h) = gc.load_texture(TextureId(0), io::Cursor::new(flo_bytes)).unwrap();
            flo_w = w;
            flo_h = h;
        });

        // Declare a sprite with our PNG file in it
        canvas.draw(|gc| {
            gc.sprite(SpriteId(0));
            gc.clear_sprite();

            let height = (flo_h as f32) / (flo_w as f32) * 128.0;

            gc.new_path();
            gc.circle(0.0, 0.0, height/2.0);
            gc.fill_texture(TextureId(0), -64.0, height/2.0, 64.0, -height/2.0);
            gc.set_texture_fill_alpha(TextureId(0), 0.75);
            gc.fill();

            gc.line_width(0.25);
            gc.stroke_color(Color::Rgba(0.0, 0.0, 0.0, 1.0));
            gc.stroke();
        });

        // Generate some random balls
        let mut balls = (0..256).into_iter().map(|_| Ball::random(SpriteId(0))).collect::<Vec<_>>();

        // Animate them
        loop {
            // Update the balls for this frame
            for ball in balls.iter_mut() {
                ball.update();
            }

            // Render the frame on layer 0
            canvas.draw(|gc| {
                gc.layer(LayerId(0));
                gc.clear_layer();
                gc.canvas_height(1000.0);
                gc.center_region(0.0, 0.0, 1000.0, 1000.0);

                for ball in balls.iter() {
                    // Render the ball's sprite at its location
                    gc.sprite_transform(SpriteTransform::Identity);
                    gc.sprite_transform(SpriteTransform::Translate(ball.x as f32, ball.y as f32));
                    gc.draw_sprite(ball.sprite_id);
                }
            });

            // Wait for the next frame
            thread::sleep(Duration::from_nanos(1_000_000_000 / 60));
        }
    });
}
