use flo_canvas::*;
use flo_render_canvas::*;

use futures::stream;
use futures::executor;

use png;

use std::io::*;
use std::path;
use std::fs::*;

///
/// Saves a file 'triangle.png' with a triangle in it
///
pub fn main() {
    executor::block_on(async {
        // Create an offscreen context
        let mut context     = initialize_offscreen_rendering().unwrap();

        // Describe what to draw
        let mut drawing     = vec![];
        drawing.clear_canvas(Color::Rgba(0.0, 0.0, 0.0, 0.0));
        drawing.canvas_height(1000.0);
        drawing.transform(Transform2D::scale(1.0, -1.0));
        drawing.center_region(0.0, 0.0, 1000.0, 1000.0);
        
        drawing.new_path();
        drawing.move_to(200.0, 200.0);
        drawing.line_to(800.0, 200.0);
        drawing.line_to(500.0, 800.0);
        drawing.line_to(200.0, 200.0);

        drawing.fill_color(Color::Rgba(0.0, 0.6, 0.8, 1.0));
        drawing.fill();

        // Render an image to bytes
        let image           = render_canvas_offscreen(&mut context, 1024, 768, 1.0, stream::iter(drawing)).await;

        // Save to a png file
        let path            = path::Path::new(r"triangle.png");
        let file            = File::create(path).unwrap();
        let ref mut writer  = BufWriter::new(file);

        let mut png_encoder = png::Encoder::new(writer, 1024, 768);
        png_encoder.set_color(png::ColorType::Rgba);
        png_encoder.set_depth(png::BitDepth::Eight);
        let mut png_writer  = png_encoder.write_header().unwrap();

        png_writer.write_image_data(&image).unwrap();
    });
}