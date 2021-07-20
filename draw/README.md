```toml
flo_draw = "0.3"
```

# flo_draw
If you want to render some 2D graphics in Rust to screen *right now* without having to deal with the usual palaver involved with setting up 
a graphics context in a UI framework, [`flo_draw`](https://crates.io/crates/flo_draw) is the crate you need.

`flo_draw` also comes with a powerful set of 2D graphics libraries and has a flexible stream-based API that can make light work of many
tasks that other libraries can make a meal of.

## Motivation

[![Screenshot](./images/beeb.png)](https://bbcmic.ro/#%7B%22v%22%3A1%2C%22program%22%3A%22MODE%200%5CnMOVE%20128%2C%20128%5CnMOVE%201280-128%2C%20128%5CnPLOT%2085%2C%201280%2F2%2C%201024-128%5CnA%24%20%3D%20GET%24%22%7D)

While building [FlowBetween](https://github.com/logicalshift/FlowBetween/), I found I needed a few when it came to rendering graphics:
a platform-agnostic API and a way to render bitmap files that's not tied to any one platform. When debugging it, I found another thing
I really wanted was a simple way to just throw up a window and start drawing graphics in it. This used to be quite simple in the 1980s
(as demonstated in the screenshot) but the rise of the GUI and 3D accelleration has made rendering graphics increasingly difficult.

`flo_draw` takes the 2D graphics crates created for FlowBetween and adds an API to take all of the hassle out of the task of making
them work.

## Basic example

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

For more information, you may be interested in the [guide](GUIDE.md), or the [examples](examples).

`flo_draw` provides an independent implementation of the rendering system used by FlowBetween, a
project to develop a vector animation editor. It was created to assist in debugging: often it's
much easier to draw a diagram, but until now it's been quite involved to write a program to tell
a computer to do that.

## --

![Wibble](./images/wibble.png) ![Mandelbrot](./images/mandelbrot.png)
![Wibble](./images/mascot.png)
