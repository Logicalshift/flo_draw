```toml
flo_draw = "0.3"
```

# flo_draw

`flo_draw` is a crate for Rust that aims to make it as easy to draw 2D graphics on screen
as it is to call `println!` to write some text to a terminal. It's based on the 2D primitives
defined in `flo_curves`, and it relies heavily on the `lyon` and `glutin` crates for tesselation
and window management.

Here's a simple example that will open up a window with a triangle in it:

```Rust
use flo_draw::*;
use flo_canvas::*;

pub fn main() {
    with_2d_graphics(|| {
        let canvas = create_canvas_window();

        canvas.draw(|gc| {
            gc.clear_canvas(Color::Rgba(0.0, 0.0, 0.0, 1.0));
            gc.canvas_height(1000.0);
            gc.center_region(0.0, 0.0, 1000.0, 1000.0);

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
    });
}
```

`flo_draw` provides an independent implementation of the rendering system used by FlowBetween, a
project to develop a vector animation editor. It was created to assist in debugging: often it's
much easier to draw a diagram, but until now it's been quite involved to write a program to tell
a computer to do that.
