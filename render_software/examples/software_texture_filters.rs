use flo_render_software::render::*;
use flo_render_software::canvas::*;

use futures::prelude::*;
use futures::stream;
use futures::executor;

use std::f64;
use std::io;
use std::sync::*;

///
/// Draws FlowBetween's mascot as a texture
///
pub fn main() {
    // Load a png file
    let flo_bytes: &[u8]    = include_bytes!["flo_drawing_on_window.png"];
    let lato                = CanvasFontFace::from_slice(include_bytes!("../test_data/Lato-Regular.ttf"));

    for filter in [TextureFilter::AlphaBlend(0.7), TextureFilter::GaussianBlur(16.0), TextureFilter::DisplacementMap(TextureId(1), 8.0, 8.0), TextureFilter::Mask(TextureId(2))] {
        // Create drawing instructions for the png
        let mut canvas = vec![];

        // Clear the canvas and set up the coordinates
        canvas.clear_canvas(Color::Rgba(1.0, 1.0, 1.0, 1.0));
        canvas.canvas_height(1000.0);
        canvas.center_region(0.0, 0.0, 1000.0, 1000.0);

        // Set up the texture that we'll render after filtering
        let (flo_w, flo_h) = canvas.load_texture(TextureId(0), io::Cursor::new(flo_bytes)).unwrap();

        let ratio   = (flo_w as f32)/(flo_h as f32);
        let height  = 1000.0 / ratio;
        let y_pos   = (1000.0-height)/2.0;

        // Create a displacement map texture as texture 1
        canvas.create_texture(TextureId(1), flo_w as _, flo_h as _, TextureFormat::Rgba);
        canvas.set_texture_bytes(TextureId(1), 0, 0, flo_w as _, flo_h as _,
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

        // Make 1000.0 units our width, and define the height based on the size of the image
        let sprite_height = 1000.0*(flo_h as f32)/(flo_w as f32);
        canvas.define_font_data(FontId(1), Arc::clone(&lato));

        // Define sprite 0 as our mask
        canvas.sprite(SpriteId(0));
        canvas.clear_sprite();
        canvas.fill_color(Color::Rgba(0.0, 0.0, 0.0, 1.0));
        canvas.set_font_size(FontId(1), 200.0);
        canvas.begin_line_layout(500.0, sprite_height/2.0-100.0, TextAlignment::Center);
        canvas.layout_text(FontId(1), "MASK".to_string());
        canvas.draw_text_layout();

        canvas.new_path();
        canvas.circle(500.0, 600.0, 100.0);
        canvas.fill();

        canvas.new_path();
        canvas.circle(500.0, 200.0, 100.0);
        canvas.fill();

        // Render the mask to texture 2
        canvas.layer(LayerId(0));
        canvas.create_texture(TextureId(2), flo_w as _, flo_h as _, TextureFormat::Rgba);
        canvas.set_texture_from_sprite(TextureId(2), SpriteId(0), 0.0, sprite_height, 1000.0, -sprite_height); // -- TODO

        // Filter the texture
        canvas.layer(LayerId(0));
        canvas.filter_texture(TextureId(0), filter);

        // Draw a rectangle filled with the filtered texture
        canvas.new_path();
        canvas.rect(0.0, y_pos, 1000.0, y_pos+height);

        canvas.fill_texture(TextureId(2), 0.0, y_pos+height as f32, 1000.0, y_pos);
        canvas.fill();

        canvas.fill_texture(TextureId(0), 0.0, y_pos+height as f32, 1000.0, y_pos);
        canvas.fill();

        let drawing = stream::iter(canvas);
        let drawing = drawing_with_laid_out_text(drawing);
        let drawing = drawing_with_text_as_paths(drawing);
        let drawing = executor::block_on(async move { drawing.collect::<Vec<_>>().await });

        // Render to the terminal window
        println!("{:?}", filter);
        render_drawing(&mut TerminalRenderTarget::new(1920, 1080), drawing.iter().cloned());
        println!();
    }
}
