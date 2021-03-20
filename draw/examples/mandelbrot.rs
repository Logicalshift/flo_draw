use flo_draw::*;
use flo_draw::canvas::*;
use flo_draw::binding::*;

use futures::prelude::*;
use futures::executor;
use futures::stream;
use num_complex::*;

use std::thread;
use std::sync::*;
use std::time::{Instant, Duration};

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
        let bounds      = bind((Complex::new(-2.5, -1.0), Complex::new(1.0, 1.0)));

        // The update number is used to synchronise other updates and interrupt drawing the mandelbrot
        let update_num  = bind(0u64);

        // Run some threads to display some different layers. We can write to layers independently on different threads
        show_title(&canvas, LayerId(100), crossfade.clone());
        show_stats(&canvas, LayerId(99), BindRef::from(&bounds), BindRef::from(&crossfade));
        show_mandelbrot(&canvas, LayerId(0), TextureId(100), BindRef::from(&width), BindRef::from(&height), BindRef::from(&bounds), BindRef::from(&crossfade), BindRef::from(&update_num));

        // Loop while there are events
        executor::block_on(async move {
            let mut events  = events;
            while let Some(evt) = events.next().await {
                match evt {
                    DrawEvent::Resize(new_width, new_height) => {
                        if width.get() != new_width as _ || height.get() != new_height as _ {
                            width.set(new_width as _);
                            height.set(new_height as _);
                            update_num.set(update_num.get() + 1);
                        }
                    }

                    _ => { }
                }
            }
        })
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

            // Blank the layer once done
            canvas.draw(|gc| {
                gc.layer(layer);
                gc.clear_layer();
            });
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
fn show_mandelbrot(canvas: &Canvas, layer: LayerId, texture: TextureId, width: BindRef<u32>, height: BindRef<u32>, bounds: BindRef<(Complex<f64>, Complex<f64>)>, crossfade: BindRef<f32>, update_num: BindRef<u64>) {
    let canvas = canvas.clone();

    thread::Builder::new()
        .name("Mandelbrot thread".into())
        .spawn(move || {
            enum Event {
                RenderBounds((u32, u32, (Complex<f64>, Complex<f64>))),
                CrossFade(f32)
            }

            let alpha           = computed(move || f32::min(f32::max(crossfade.get()-1.0, 0.0),1.0));
            let alpha           = BindRef::from(alpha);
            let mut texture_w   = width.get();
            let mut texture_h   = height.get();

            // The render bounds are used to determine when we start to re-render the mandelbrot set
            let render_bounds   = computed(move || (width.get(), height.get(), bounds.get()));

            // Events either start rendering a new frame or changing the crossfade
            let render_bounds   = follow(render_bounds).map(|bounds| Event::RenderBounds(bounds)).boxed();
            let crossfade       = follow(alpha.clone()).map(|xfade| Event::CrossFade(xfade)).boxed();

            let events          = stream::select_all(vec![render_bounds, crossfade]);

            // Wait for events and render the mandelbrot set as they arrive
            executor::block_on(async move {
                let mut events = events;

                while let Some(evt) = events.next().await {
                    match evt {
                        Event::RenderBounds((new_width, new_height, new_bounds)) => {
                            texture_w = new_width;
                            texture_h = new_height;

                            // Create the texture for this width and height
                            canvas.draw(|gc| {
                                gc.layer(layer);
                                gc.create_texture(texture, texture_w, texture_h, TextureFormat::Rgba);
                                gc.set_texture_fill_alpha(texture, alpha.get());
                            });

                            // Fill it in with the current bounds
                            draw_mandelbrot(&canvas, layer, texture, new_bounds, texture_w, texture_h, &alpha, &update_num);
                        }

                        Event::CrossFade(new_alpha) => {
                            // Redraw the texture with the new alpha
                            canvas.draw(|gc| {
                                gc.layer(layer);
                                gc.clear_layer();
                                gc.set_texture_fill_alpha(texture, new_alpha);

                                gc.canvas_height(texture_h as _);
                                gc.center_region(0.0, 0.0, texture_w as _, texture_h as _);

                                gc.new_path();
                                gc.rect(0.0, 0.0, texture_w as _, texture_h as _);
                                gc.fill_texture(texture, 0.0, 0.0, texture_w as _, texture_h as _);
                                gc.fill();
                            });
                        }
                    }
                }
            });
        })
        .unwrap();
}

///
/// Draws the mandelbrot set within a specified set of bounds
///
fn draw_mandelbrot(canvas: &Canvas, layer: LayerId, texture: TextureId, (min, max): (Complex<f64>, Complex<f64>), width: u32, height: u32, alpha: &BindRef<f32>, update_num: &BindRef<u64>) {
    // Create a vector for the pixels in the mandelbrot set
    let mut pixels  = vec![0u8; (width*height*4) as usize];
    let mut pos     = 0;
    let update      = update_num.get();

    let mut start_time = Instant::now();

    // Render each pixel in turn
    for y in 0..height {
        let y = y as f64;
        let y = y / (height as f64);
        let y = (max.im - min.im) * y + min.im;

        for x in 0..width {
            let x = x as f64;
            let x = x / (width as f64);
            let x = (max.re - min.re) * x + min.re;

            let c               = Complex::new(x, y);
            let cycles          = count_cycles(c, 256);
            let (r, g, b, a)    = color_for_cycles(cycles);

            pixels[pos+0]       = r;
            pixels[pos+1]       = g;
            pixels[pos+2]       = b;
            pixels[pos+3]       = a;

            pos                 += 4;
        }

        // Stop if there's an update to the state we're rendering
        if update_num.get() != update {
            return;
        }

        // Draw the story so far every 50ms
        if Instant::now().duration_since(start_time) > Duration::from_millis(50) {
            let intermediate_pixels = Arc::new(pixels.clone());
            canvas.draw(move |gc| {
                gc.layer(layer);
                gc.clear_layer();
                gc.create_texture(texture, width, height, TextureFormat::Rgba);
                gc.set_texture_bytes(texture, 0, 0, width, height, intermediate_pixels);
                gc.set_texture_fill_alpha(texture, alpha.get());

                gc.canvas_height(height as _);
                gc.center_region(0.0, 0.0, width as _, height as _);

                gc.new_path();
                gc.rect(0.0, 0.0, width as _, height as _);
                gc.fill_texture(texture, 0.0, 0.0, width as _, height as _);
                gc.fill();
            });

            start_time = Instant::now();
        }
    }

    // Draw to the texture
    canvas.draw(move |gc| {
        gc.create_texture(texture, width, height, TextureFormat::Rgba);
        gc.set_texture_bytes(texture, 0, 0, width, height, Arc::new(pixels));

        gc.layer(layer);
        gc.clear_layer();
        gc.set_texture_fill_alpha(texture, alpha.get());

        gc.canvas_height(height as _);
        gc.center_region(0.0, 0.0, width as _, height as _);

        gc.new_path();
        gc.rect(0.0, 0.0, width as _, height as _);
        gc.fill_texture(texture, 0.0, 0.0, width as _, height as _);
        gc.fill();
    });
}

///
/// Counts the number of cycles (up to a maximum count) at a particular pixel
///
#[inline]
fn count_cycles(c: Complex<f64>, max_count: usize) -> usize {
    let mut z       = Complex::new(0.0, 0.0);
    let mut count   = 0;

    while count < max_count && (z.re*z.re + z.im*z.im) < 2.0*2.0 {
        z       = z*z + c;
        count   = count + 1;
    }

    count
}

///
/// Returns the colour to use for a particular number of cycles
///
#[inline]
fn color_for_cycles(num_cycles: usize) -> (u8, u8, u8, u8) {
    let col_val = (num_cycles%256) as u8;

    (col_val, col_val, col_val, 255)
}
