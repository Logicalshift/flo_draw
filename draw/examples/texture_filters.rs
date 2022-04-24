use flo_draw::*;
use flo_draw::canvas::*;

use futures::prelude::*;
use futures::executor;

use std::f64;
use std::io;
use std::sync::*;

///
/// Simple example that displays a canvas window and renders an image from a png file
///
pub fn main() {
    // 'with_2d_graphics' is used to support operating systems that can't run event loops anywhere other than the main thread
    with_2d_graphics(|| {
        // Load a png file
        let flo_bytes: &[u8]    = include_bytes!["flo_drawing_on_window.png"];
        let lato                = CanvasFontFace::from_slice(include_bytes!["Lato-Regular.ttf"]);

        // Create a window
        let (canvas, events)    = create_drawing_window_with_events("Filtered texture");

        // Set up the canvas
        let mut flo_w = 0;
        let mut flo_h = 0;
        canvas.draw(|gc| {
            // Clear the canvas and set up the coordinates
            gc.clear_canvas(Color::Rgba(1.0, 1.0, 1.0, 1.0));
            gc.canvas_height(1000.0);
            gc.center_region(0.0, 0.0, 1000.0, 1000.0);

            // Load a font to display what's going on
            gc.define_font_data(FontId(1), Arc::clone(&lato));

            // Load the texture into TextureId(0)
            (flo_w, flo_h) = gc.load_texture(TextureId(0), io::Cursor::new(flo_bytes)).unwrap();
        });

        executor::block_on(async move {
            let mut events = events;
            let mut filter = 0;

            loop {
                // Render the next filter
                canvas.draw(|gc| {
                    gc.layer(LayerId(1));
                    gc.clear_layer();

                    // Instructions
                    gc.fill_color(Color::Rgba(0.0, 0.0, 0.2, 0.8));
                    gc.set_font_size(FontId(1), 24.0);
                    gc.begin_line_layout(500.0, 20.0, TextAlignment::Center);
                    gc.layout_text(FontId(1), "Press space for next filter".to_string());
                    gc.draw_text_layout();

                    // Filter name
                    gc.set_font_size(FontId(1), 18.0);
                    gc.begin_line_layout(990.0, 20.0, TextAlignment::Right);

                    let name = match filter {
                        0 => "Gaussian blur",
                        1 => "Alpha blend",
                        2 => "Mask",
                        3 => "Displacement map",

                        _ => "Unknown filter"
                    };
                    gc.layout_text(FontId(1), name.to_string());
                    gc.draw_text_layout();

                    // Draw the texture with a filter
                    gc.layer(LayerId(0));
                    gc.clear_layer();

                    gc.copy_texture(TextureId(0), TextureId(1));

                    match filter {
                        // Gaussian blur
                        0 => gc.filter_texture(TextureId(1), TextureFilter::GaussianBlur(16.0)),

                        // Alpha blend
                        1 => gc.filter_texture(TextureId(1), TextureFilter::AlphaBlend(0.5)),

                        // Mask
                        2 => {
                            let sprite_height = 1000.0*(flo_h as f32)/(flo_w as f32);

                            // Define sprite 0 as our mask
                            gc.sprite(SpriteId(0));
                            gc.clear_sprite();
                            gc.set_font_size(FontId(1), 200.0);
                            gc.begin_line_layout(500.0, sprite_height/2.0-100.0, TextAlignment::Center);
                            gc.layout_text(FontId(1), "MASK".to_string());
                            gc.draw_text_layout();

                            // Back to layer 0, then render the sprite to a texture that's the same size as our input texture
                            gc.layer(LayerId(0));
                            gc.create_texture(TextureId(2), flo_w as _, flo_h as _, TextureFormat::Rgba);
                            gc.set_texture_from_sprite(TextureId(2), SpriteId(0), 0.0, sprite_height, 1000.0, -sprite_height);

                            // Filter the texture with the stencil we just created
                            gc.filter_texture(TextureId(1), TextureFilter::Mask(TextureId(2)));
                        }

                        // Displacement map
                        3 => {
                            // Define a texture to use as the displacement map (the red and green channels are the x and y displacement as a proportion of the scaling factor)
                            gc.create_texture(TextureId(2), flo_w as _, flo_h as _, TextureFormat::Rgba);
                            gc.set_texture_bytes(TextureId(2), 0, 0, flo_w as _, flo_h as _,
                                Arc::new((0..(flo_w*flo_h)).into_iter()
                                    .flat_map(|pixel_num| {
                                        let x_pos       = pixel_num % flo_w;
                                        let y_pos       = pixel_num / flo_w;

                                        let x_factor    = (x_pos as f64) / (flo_w as f64);
                                        let y_factor    = (y_pos as f64) / (flo_h as f64);
                                        let x_factor    = x_factor * 2.0 * f64::consts::PI;
                                        let y_factor    = y_factor * 2.0 * f64::consts::PI;
                                        let x_factor    = x_factor * 8.0;
                                        let y_factor    = y_factor * 7.0;

                                        let x_seq       = (x_factor.sin() + 1.0)/2.0;
                                        let y_seq       = (y_factor.cos() + 1.0)/2.0;

                                        [(y_seq*255.0) as u8, (x_seq*255.0) as u8, 0, 255]
                                    })
                                    .collect::<Vec<_>>()));

                            // Distort the texture with the filter we just created
                            gc.filter_texture(TextureId(1), TextureFilter::DisplacementMap(TextureId(2), 8.0, 8.0));
                        }

                        _ => { }
                    }

                    // Draw a rectangle...
                    let ratio   = (flo_w as f32)/(flo_h as f32);
                    let height  = 1000.0 / ratio;
                    let y_pos   = (1000.0-height)/2.0;

                    gc.new_path();
                    gc.rect(0.0, y_pos, 1000.0, y_pos+height);

                    // Fill with the texture we just filtered
                    gc.fill_texture(TextureId(1), 0.0, y_pos+height as f32, 1000.0, y_pos);
                    gc.fill();
                });

                // Wait for the user to hit the space bar
                loop {
                    let next_event = events.next().await;
                    if next_event.is_none() { return; }

                    if let Some(DrawEvent::KeyDown(_, Some(key))) = next_event {
                        if key == Key::KeySpace {
                            break;
                        }
                    }
                }

                // Move to the next filter
                filter = (filter + 1) % 4;
            }
        });
    });
}
