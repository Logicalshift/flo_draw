use flo_canvas::*;
use flo_render_canvas::*;

use futures::stream;
use futures::executor;

use png::*;

use std::io::*;
use std::path::*;
use std::fs::*;

///
/// Saves a file 'triangle.png' with a triangle in it
///
pub fn main() {
    executor::block_on(async {
        // Create an offscreen context
        let mut context     = initialize_offscreen_rendering().unwrap();

        // Render an image to bytes
        use Draw::*;
        let image           = render_canvas_offscreen(&mut context, 1024, 768, 1.0, stream::iter(vec![
            ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 0.0)),
            CanvasHeight(1000.0),
            MultiplyTransform(Transform2D::scale(1.0, -1.0)),
            CenterRegion((0.0, 0.0), (1000.0, 1000.0)),

            NewPath,
            Move(200.0, 200.0),
            Line(800.0, 200.0),
            Line(500.0, 800.0),
            Line(200.0, 200.0),

            FillColor(Color::Rgba(0.0, 0.6, 0.8, 1.0)),
            Fill
        ])).await;

        // Save to a png file
        let path            = Path::new(r"triangle.png");
        let file            = File::create(path).unwrap();
        let ref mut writer  = BufWriter::new(file);

        let mut png_encoder = png::Encoder::new(writer, 1024, 768);
        png_encoder.set(png::ColorType::RGBA).set(png::BitDepth::Eight);
        let mut png_writer  = png_encoder.write_header().unwrap();

        png_writer.write_image_data(&image).unwrap();
    });
}