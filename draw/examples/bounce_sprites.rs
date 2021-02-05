use flo_draw::*;
use flo_canvas::*;

use rand::*;

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
    pub fn random(sprite_id: SpriteId, canvas: &Canvas) -> Ball {
        // Decide on how the ball is rendered
        let col     = Color::Hsluv(random::<f32>()*360.0, random::<f32>()*100.0, random::<f32>()*75.0 + 25.0, 1.0);
        let radius  = random::<f64>() * 16.0 + 16.0;

        // Declare the sprite
        canvas.draw(|gc| {
            gc.sprite(sprite_id);
            gc.clear_sprite();

            gc.new_path();
            gc.circle(0.0,0.0, radius as f32);
            gc.fill_color(col);
            gc.fill();
        });

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
/// Bouncing ball example that uses sprites to improve performance
///
/// bounce.rs renders the paths every frame, so each circle has to be re-tessellated every time. This uses
/// sprites so that the paths are only tessellated once, which reduces the CPU requirements considerably.
///
pub fn main() {
    // 'with_2d_graphics' is used to support operating systems that can't run event loops anywhere other than the main thread
    with_2d_graphics(|| {
        // Create a window with a canvas to draw on
        let canvas = create_canvas_window("Bouncing sprites");

        // Clear the canvas to set a background colour
        canvas.draw(|gc| {
            gc.clear_canvas(Color::Rgba(0.6, 0.7, 0.8, 1.0));
        });

        // Generate some random balls
        let mut balls = (0..256).into_iter().map(|idx| Ball::random(SpriteId(idx), &canvas)).collect::<Vec<_>>();

        // Animate them
        loop {
            // Update the balls for this frame
            for ball in balls.iter_mut() {
                ball.update();
            }

            // Render the frame on layer 0
            canvas.draw(|gc| {
                gc.layer(0);
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
