use flo_draw::*;
use flo_canvas::*;
use flo_stream::*;

use rand::*;

use futures::prelude::*;
use futures::stream;
use futures::executor;
use futures_timer::*;

use std::f64;
use std::time::*;

///
/// Demonstration of `flo_draw` that illustrates a way to implement a simple game
///
pub fn main() {
    with_2d_graphics(|| {
        executor::block_on(async {
            // Set up
            let (canvas, events) = create_canvas_window_with_events("Vectoroids");

            // Create a tick generator
            let tick_stream = tick_stream();

            // Combine the tick generator into the events stream
            let events      = events.map(|evt| VectorEvent::DrawEvent(evt));
            let mut events  = stream::select(events, tick_stream);

            // Set up the canvas by declaring the sprites and the background
            let ship_sprite     = SpriteId(0);
            let bullet_sprite   = SpriteId(1);
            let roid_sprite     = SpriteId(2);
            canvas.draw(|gc| {
                gc.clear_canvas(Color::Rgba(0.1, 0.1, 0.1, 1.0));

                // Game area is a 1000x1000 square
                gc.canvas_height(1000.0);
                gc.center_region(0.0, 0.0, 1000.0, 1000.0);

                // Ship is just a triangle
                gc.sprite(ship_sprite);
                gc.clear_sprite();

                gc.new_path();
                gc.move_to(-10.0, -8.0);
                gc.line_to(0.0, 12.0);
                gc.line_to(10.0, -8.0);
                gc.line_to(8.0, -6.0);
                gc.line_to(-8.0, -6.0);
                gc.line_to(-10.0, -8.0);

                gc.line_width(2.0);
                gc.stroke_color(Color::Rgba(0.8, 0.7, 0.0, 1.0));
                gc.stroke();

                // Bullet is a square
                gc.sprite(bullet_sprite);
                gc.clear_sprite();

                gc.new_path();
                gc.rect(-1.0, -1.0, 1.0, 1.0);
                gc.fill_color(Color::Rgba(1.0, 0.8, 0.0, 1.0));
                gc.fill();

                // 'roids are an irregular shape
                gc.sprite(roid_sprite);
                gc.clear_sprite();

                gc.new_path();
                gc.move_to(0.0, -15.0);
                gc.line_to(-15.0, -25.0);
                gc.line_to(-35.0, -15.0);
                gc.line_to(-20.0, 0.0);
                gc.line_to(-25.0, 5.0);
                gc.line_to(-10.0, 20.0);
                gc.line_to(5.0, 25.0);
                gc.line_to(30.0, 5.0);
                gc.line_to(20.0, -15.0);
                gc.line_to(10.0, -25.0);
                gc.line_to(0.0, -15.0);

                gc.line_width(3.0);
                gc.stroke_color(Color::Rgba(0.6, 0.5, 0.0, 1.0));
                gc.stroke();

                // Background is a random starscape made of squares
                gc.layer(0);
                gc.clear_layer();

                gc.fill_color(Color::Rgba(0.7, 0.7, 0.8, 1.0));
                for _star in 0..200 {
                    let x = random::<f32>() * 1000.0;
                    let y = random::<f32>() * 1000.0;

                    gc.rect(x-1.0, y-1.0, x+1.0, y+1.0);
                    gc.fill();
                }
            });

            // Run the game by processing events
            let mut game_state = GameState::new(ship_sprite, roid_sprite);

            while let Some(event) = events.next().await {
                match event {
                    VectorEvent::Tick   => {
                        // Update the game state
                        game_state.tick();

                        // Draw the game on layer 1
                        canvas.draw(|gc| {
                            gc.layer(1);
                            gc.clear_layer();

                            game_state.draw(gc);
                        });
                    }

                    VectorEvent::DrawEvent(DrawEvent::KeyDown(_, Some(Key::KeyLeft))) => {
                        game_state.ship.rotation = (360.0) / 60.0;
                    }
                    VectorEvent::DrawEvent(DrawEvent::KeyDown(_, Some(Key::KeyRight))) => {
                        game_state.ship.rotation = -(360.0) / 60.0;
                    }
                    VectorEvent::DrawEvent(DrawEvent::KeyUp(_, Some(Key::KeyLeft))) |
                    VectorEvent::DrawEvent(DrawEvent::KeyUp(_, Some(Key::KeyRight))) => {
                        game_state.ship.rotation = 0.0;
                    }

                    VectorEvent::DrawEvent(DrawEvent::KeyDown(_, Some(Key::KeyUp))) => {
                        game_state.ship.thrust = 0.3;
                    }
                    VectorEvent::DrawEvent(DrawEvent::KeyUp(_, Some(Key::KeyUp))) => {
                        game_state.ship.thrust = 0.0;
                    }

                    _ => { /* Other events are ignored */ }
                }
            }
        });
    });
}

///
/// Events processed by the game
///
enum VectorEvent {
    Tick,
    DrawEvent(DrawEvent)
}

///
/// Represents the state of a game
///
struct GameState {
    ship:           Ship,

    roid_sprite:    SpriteId,
    roids:          Vec<Roid>
}

///
/// Represents the state of the player's ship
///
struct Ship {
    sprite:     SpriteId,

    x:          f64,
    y:          f64,
    angle:      f64,
    vel_x:      f64,
    vel_y:      f64,
    rotation:   f64,
    thrust:     f64
}

///
/// Represents the state of a 'roid
///
struct Roid {
    sprite:     SpriteId,

    x:          f64,
    y:          f64,
    angle:      f64,
    rotation:   f64,
    vel_x:      f64,
    vel_y:      f64,
}

impl GameState {
    ///
    /// Creates a new game state
    ///
    pub fn new(ship_sprite: SpriteId, roid_sprite: SpriteId) -> GameState {
        GameState {
            ship:           Ship::new(ship_sprite),
            roid_sprite:    roid_sprite,
            roids:          (0..20).into_iter().map(|_| Roid::new(roid_sprite)).collect()
        }
    }

    ///
    /// Updates the game state after a tick
    ///
    pub fn tick(&mut self) {
        self.ship.tick();
        self.roids.iter_mut().for_each(|roid| roid.tick());
    }

    pub fn draw(&self, gc: &mut dyn GraphicsPrimitives) {
        self.roids.iter().for_each(|roid| roid.draw(gc));
        self.ship.draw(gc);
    }
}

impl Ship {
    ///
    /// Creates a new ship state
    ///
    pub fn new(sprite: SpriteId) -> Ship {
        Ship {
            sprite:     sprite,
            x:          500.0,
            y:          500.0,
            vel_x:      0.0,
            vel_y:      0.0,
            angle:      0.0,
            rotation:   0.0,
            thrust:     0.0
        }        
    }

    ///
    /// Updates the ship state after a tick
    ///
    pub fn tick(&mut self) {
        // Move the ship
        self.x      += self.vel_x;
        self.y      += self.vel_y;
        self.angle  += self.rotation;
        self.angle  = self.angle % 360.0;

        // Clip to the play area
        if self.x < 0.0 { self.x = 1000.0 };
        if self.y < 0.0 { self.y = 1000.0 };

        if self.x > 1000.0 { self.x = 0.0 };
        if self.y > 1000.0 { self.y = 0.0 };

        // Apply thrust
        let (acc_x, acc_y) = Transform2D::rotate_degrees(self.angle as _).transform_point(0.0, self.thrust as _);
        self.vel_x += acc_x as f64;
        self.vel_y += acc_y as f64;

        // Friction
        self.vel_x *= 0.99;
        self.vel_y *= 0.99;
    }

    pub fn draw(&self, gc: &mut dyn GraphicsPrimitives) {
        gc.sprite_transform(SpriteTransform::Identity);
        gc.sprite_transform(SpriteTransform::Translate(self.x as _, self.y as _));
        gc.sprite_transform(SpriteTransform::Rotate(self.angle as _));
        gc.draw_sprite(self.sprite);
    }
}


impl Roid {
    ///
    /// Creates a new 'roid state
    ///
    pub fn new(sprite: SpriteId) -> Roid {
        Roid {
            sprite:     sprite,
            x:          random::<f64>() * 1000.0,
            y:          random::<f64>() * 1000.0,
            vel_x:      random::<f64>() * 3.0 - 1.5,
            vel_y:      random::<f64>() * 3.0 - 1.5,
            angle:      random::<f64>() * 360.0,
            rotation:   random::<f64>() * 8.0 - 4.0
        }        
    }

    ///
    /// Updates the 'roid state after a tick
    ///
    pub fn tick(&mut self) {
        // Move the 'roid
        self.x      += self.vel_x;
        self.y      += self.vel_y;
        self.angle  += self.rotation;
        self.angle  = self.angle % 360.0;

        // Clip to the play area
        if self.x < 0.0 { self.x = 1000.0 };
        if self.y < 0.0 { self.y = 1000.0 };

        if self.x > 1000.0 { self.x = 0.0 };
        if self.y > 1000.0 { self.y = 0.0 };
    }

    pub fn draw(&self, gc: &mut dyn GraphicsPrimitives) {
        gc.sprite_transform(SpriteTransform::Identity);
        gc.sprite_transform(SpriteTransform::Translate(self.x as _, self.y as _));
        gc.sprite_transform(SpriteTransform::Rotate(self.angle as _));
        gc.draw_sprite(self.sprite);
    }
}

///
/// A stream that generates a 'tick' event every time the game state should update
///
fn tick_stream() -> impl Send+Unpin+Stream<Item=VectorEvent> {
    generator_stream(|yield_value| async move {
        // Set up the clock
        let start_time          = Instant::now();
        let mut last_time       = Duration::from_millis(0);

        // We limit to a certain number of ticks per callback (in case the task is suspended or stuck for a prolonged period of time)
        let max_ticks_per_call  = 5;

        // Ticks are generated 60 times a second
        let tick_length         = Duration::from_nanos(1_000_000_000 / 60);

        loop {
            // Time that has elapsed since the last tick
            let elapsed         = start_time.elapsed() - last_time;

            // Time remaining
            let mut remaining   = elapsed;
            let mut num_ticks   = 0;
            while remaining >= tick_length {
                if num_ticks < max_ticks_per_call {
                    // Generate the tick
                    yield_value(VectorEvent::Tick).await;
                    num_ticks += 1;
                }

                // Remove from the remaining time, and update the last tick time
                remaining -= tick_length;
                last_time += tick_length;
            }

            // Wait for half a tick before generating more ticks
            let next_time = tick_length - remaining;
            let wait_time = Duration::min(tick_length / 2, next_time);

            Delay::new(wait_time).await;
        }
    }.boxed())
}
