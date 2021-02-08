# flo_draw

This is a set of libraries that provide a 2D rendering framework for Rust. It provides on and off-screen rendering and
an abstraction API.

* `flo_draw` is a library that renders 2D graphics on-screen via glutin
* `flo_canvas` provides a way to describe 2D drawing operations without being tied to any particular rendering implementation
* `flo_render` is an abstraction API that converts low-level rendering instructions to a graphics API (OpenGL and Metal are supported)
* `flo_render_canvas` converts the instructions described in `flo_canvas` to instructions for `flo_render` (using lyon for the tessellation)
* `flo_render_gl_offscreen` helps `flo_render` by providing system-specific initialisation instructions for offscreen rendering

There are some other implementations of the `flo_canvas` protocol that are not yet packaged up conveniently: in particular,
`canvas.js` allows rendering to an HTML canvas, and FlowBetween contains implementations for Quartz and Cairo.

# Getting started

The `flo_draw` library is the best place to start, it provides a very easy way to render things on-screen:

```Rust
use flo_draw::*;
use flo_canvas::*;

pub fn main() {
    with_2d_graphics(|| {
        let canvas = create_canvas_window("Hello, triangle");

        canvas.draw(|gc| {
            gc.clear_canvas(Color::Rgba(0.0, 0.4, 0.4, 1.0));
            gc.canvas_height(1000.0);
            gc.center_region(0.0, 0.0, 1000.0, 1000.0);

            gc.new_path();
            gc.move_to(200.0, 200.0);
            gc.line_to(800.0, 200.0);
            gc.line_to(500.0, 800.0);
            gc.line_to(200.0, 200.0);

            gc.fill_color(Color::Rgba(0.0, 0.0, 0.8, 1.0));
            gc.fill();
        });
    });
}
```

# Examples

See the examples folder in the `draw` and `render_canvas` subdirectories for some more things that can be done with the library.

![Screenshot](./images/bounce.png)

* `cargo run --example canvas_window` - displays a basic window
* `cargo run --example bounce_sprites` - animates some bouncing balls
* `cargo run --example follow_mouse` - demonstrates event handling by tracking the mouse around
* `cargo run --example vectoroids` - more involved example of event handling with an incomplete game (arrow keys to move, space to fire)
* `cargo run --example png_triangle` - renders a triangle to a png file

# Companion crates

`flo_draw` was developed alongside several other crates, which may be of interest when developing software that uses the canvas:

* `flo_curves` provides a lot of functionality for manipulating bezier curves.
* `flo_stream` provides pubsub and generator streams, which are useful for distributing events around an application.
    (See the vectoroids example for a way to use a generator stream as a game clock)
* `desync` provides a simpler way to write asynchronous code than traditional threads
* `flo_binding` provides a way to convert between state changes and message streams, used in `flo_draw` to update the window configuration

