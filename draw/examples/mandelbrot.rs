use flo_draw::*;
use flo_draw::canvas::*;
use flo_draw::binding::*;

use futures::prelude::*;
use futures::executor;
use num_complex::*;

use std::thread;
use std::sync::*;
use std::time::{Duration};

///
/// Renders the mandelbrot set, demonstrates how to render from multiple threads and communicate with bindings
///
/// See the `flo_binding` library for some details about how bindings work
///
pub fn main() {
    with_2d_graphics(|| {
        let (canvas, events)    = create_canvas_window_with_events("Mandelbrot set");
        let lato                = CanvasFontFace::from_slice(include_bytes!("Lato-Regular.ttf"));
        let lato_bold           = CanvasFontFace::from_slice(include_bytes!("Lato-Bold.ttf"));

        // Initialise the canvas
        canvas.draw(|gc| {
            gc.clear_canvas(Color::Rgba(0.9, 0.9, 1.0, 1.0));
            gc.define_font_data(FontId(1), Arc::clone(&lato));
            gc.define_font_data(FontId(2), Arc::clone(&lato_bold));
        });

        // Create some bindings that represent our state
        let width       = bind(1024u32);
        let height      = bind(768u32);
        let crossfade   = bind(0.0);
        let bounds      = bind((Complex::new(-1.0, -1.0), Complex::new(1.0, 1.0)));

        // Run some threads to display some different layers. We can write to layers independently on different threads
        show_title(&canvas, LayerId(100), crossfade.clone());
        show_stats(&canvas, LayerId(99), BindRef::from(&bounds), BindRef::from(&crossfade));
        show_mandelbrot(&canvas, LayerId(0), TextureId(100), BindRef::from(&width), BindRef::from(&height), BindRef::from(&bounds), BindRef::from(&crossfade));
    })
}

///
/// Runs a thread that shows the title 
///
fn show_title(canvas: &Canvas, layer: LayerId, crossfade: Binding<f32>) {
    let canvas = canvas.clone();

    thread::Builder::new()
        .name("Title thread".into())
        .spawn(move || {
            // Draw the title with a cross-fade
            for fade in 0..=180 {
                // Update the crossfade factor for the other threads. Fade goes from 0.0 to 2.0
                let fade = (fade as f32) / 90.0;
                crossfade.set(fade);

                // Draw the title, with a cross fade to show the mandelbrot set
                canvas.draw(|gc| {
                    gc.layer(layer);
                    gc.clear_layer();

                    gc.canvas_height(1000.0);
                    gc.center_region(0.0, 0.0, 1000.0, 1000.0);

                    let title_fade = (2.0-fade) - 0.5;
                    let title_fade = f32::min(f32::max(title_fade, 0.0), 1.0);

                    // Title card
                    gc.fill_color(Color::Rgba(0.0, 0.0, 0.6, title_fade));
                    gc.set_font_size(FontId(2), 36.0);
                    gc.begin_line_layout(500.0, 482.0 + (title_fade*4.0), TextAlignment::Center);
                    gc.layout_text(FontId(2), "Mandelbrot set".into());
                    gc.draw_text_layout();

                    gc.set_font_size(FontId(1), 16.0);
                    gc.begin_line_layout(500.0, 430.0 - (title_fade*4.0), TextAlignment::Center);
                    gc.layout_text(FontId(1), "A flo_draw demonstration".into());
                    gc.draw_text_layout();

                    gc.begin_line_layout(500.0, 400.0, TextAlignment::Center);
                    gc.layout_text(FontId(1), "Written by Andrew Hunter".into());
                    gc.draw_text_layout();
                });

                // Fade at 60fps
                thread::sleep(Duration::from_nanos(1_000_000_000 / 60));
            }
        })
        .unwrap();
}

///
/// Runs a thread that displays some statistics for the current rendering
///
fn show_stats(canvas: &Canvas, layer: LayerId, bounds: BindRef<(Complex<f64>, Complex<f64>)>, crossfade: BindRef<f32>) {
    let canvas = canvas.clone();
}


///
/// Runs a thread that renders the mandelbrot set whenever the bindings change 
///
fn show_mandelbrot(canvas: &Canvas, layer: LayerId, texture: TextureId, width: BindRef<u32>, height: BindRef<u32>, bounds: BindRef<(Complex<f64>, Complex<f64>)>, crossfade: BindRef<f32>) {
    let canvas = canvas.clone();
}
