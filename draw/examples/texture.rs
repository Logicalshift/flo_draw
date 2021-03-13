use flo_draw::*;
use flo_draw::canvas::*;

use png;
use std::sync::*;

///
/// Simple example that displays a canvas window and renders a triangle
///
pub fn main() {
    // 'with_2d_graphics' is used to support operating systems that can't run event loops anywhere other than the main thread
    with_2d_graphics(|| {
        // Load a png file
        let flo_bytes: &[u8]            = include_bytes!["flo_and_carrot.png"];
        let flo_decoder                 = png::Decoder::new(flo_bytes);
        let (flo_info, mut flo_reader)  = flo_decoder.read_info().unwrap();
        let mut flo_data                = vec![0; flo_info.buffer_size()];
        flo_reader.next_frame(&mut flo_data).unwrap();
        let flo_data                    = Arc::new(flo_data);
        let (flo_w, flo_h)              = (flo_info.width, flo_info.height);

        // Create a window
        let canvas = create_canvas_window("Flo with carrot");

        // Render a triangle to it
        canvas.draw(|gc| {
            // Clear the canvas and set up the coordinates
            gc.clear_canvas(Color::Rgba(1.0, 1.0, 1.0, 1.0));
            gc.canvas_height(1000.0);
            gc.center_region(0.0, 0.0, 1000.0, 1000.0);

            // Set up the texture
            gc.create_texture(TextureId(0), flo_w as _, flo_h as _, TextureFormat::Rgba);
            gc.set_texture_bytes(TextureId(0), 0, 0, flo_w as _, flo_h as _, Arc::clone(&flo_data));

            // Draw a rectangle...
            gc.new_path();
            gc.rect(0.0, 0.0, 1000.0, 1000.0);

            // Fill with the texture we just loaded
            gc.fill_texture(TextureId(0), 0.0, flo_h as _, flo_w as _, 0.0);
            gc.fill();
        });
    });
}
