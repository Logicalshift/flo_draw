use flo_render_software::render::*;
use flo_render_software::canvas::*;

use std::f32;
use std::fs::File;
use std::path::PathBuf;
use flo_render_software::draw::{CanvasDrawing, CanvasDrawingRegionRenderer};
use flo_render_software::pixel::{F32CmykaPixel, F32LinearPixel, Pixel, ToRgbaPremultipliedPixels};
use flo_render_software::scanplan::ShardScanPlanner;
use tiff::encoder::TiffEncoder;

///
/// Draws a simple linear gradient
///
pub fn main() {
    // Create drawing instructions for the png
    let mut canvas = vec![];
    
    let angle = (30.0 / 360.0) * (2.0 * f32::consts::PI);
    
    // Clear the canvas and set up the coordinates
    canvas.clear_canvas(Color::Cmyka(0.0, 0.0, 0.0, 0.0, 1.0));
    canvas.canvas_height(1000.0);
    canvas.center_region(0.0, 0.0, 1000.0, 1000.0);
    
    canvas.layer(LayerId(0));
    canvas.clear_layer();
    
    // Set up the canvas
    canvas.canvas_height(1000.0);
    canvas.center_region(0.0, 0.0, 1000.0, 1000.0);
    
    // Set up a gradient
    canvas.create_gradient(GradientId(1), Color::Cmyka(0.8, 0.0, 0.0, 0.0, 1.0));
    canvas.gradient_stop(GradientId(1), 0.33, Color::Cmyka(0.3, 0.8, 0.0, 0.0, 1.0));
    canvas.gradient_stop(GradientId(1), 0.66, Color::Cmyka(0.0, 0.3, 0.8, 0.9, 1.0));
    canvas.gradient_stop(GradientId(1), 1.0, Color::Cmyka(0.6, 0.3, 0.9, 0.0, 1.0));
    
    let x1 = 500.0 - 300.0 * f32::cos(angle);
    let y1 = 500.0 - 300.0 * f32::sin(angle);
    let x2 = 500.0 + 300.0 * f32::cos(angle);
    let y2 = 500.0 + 300.0 * f32::sin(angle);
    
    // Draw a circle using the gradient
    canvas.new_path();
    canvas.circle(500.0, 500.0, 250.0);
    canvas.fill_gradient(GradientId(1), x1, y1, x2, y2);
    canvas.fill();
    
    canvas.line_width(4.0);
    canvas.stroke_color(Color::Cmyka(0.0, 0.0, 0.0, 1.0, 1.0));
    canvas.stroke();
    
    // Draw indicators where the gradient is moving between
    canvas.line_width(1.0);
    
    canvas.new_path();
    canvas.circle(x1, y1, 8.0);
    canvas.stroke();
    
    canvas.new_path();
    canvas.circle(x2, y2, 8.0);
    canvas.stroke();
    
    // Render to the terminal window
    // render_drawing(&mut TerminalRenderTarget::new(1920, 1080), canvas.iter().cloned());
    
    let mut canvas_drawing = CanvasDrawing::<F32CmykaPixel, 5>::empty();
    canvas_drawing.draw(canvas);
    
    let width = 1920;
    let height = 1080;
    
    let renderer = CanvasDrawingRegionRenderer::new(ShardScanPlanner::default(), ScanlineRenderer::new(canvas_drawing.program_runner(height as _)), height);
    
    let renderer = F32CmykaFrameRenderer::new(renderer);
    let frame_size = GammaFrameSize { width, height, gamma: 2.2 };
    let mut pixel_data = vec![F32CmykaPixel::default(); width * height];
    
    renderer.render(&frame_size, &canvas_drawing, pixel_data.as_mut_slice());
    println!("wat {}", pixel_data.iter().flat_map(|c| c.to_cmyk()).reduce(f32::max).unwrap());
    
    let file = File::create("cmyk_software_gradient.tiff").unwrap();
    let mut tiff_enc = TiffEncoder::new_big(file).unwrap();
    let mut img_enc = tiff_enc.new_image::<tiff::encoder::colortype::CMYKA8>(width as u32, height as u32).unwrap();
    let es: &[u8] = &[2u8];
    img_enc.encoder().write_tag(tiff::tags::Tag::ExtraSamples, es).unwrap();
    
    let pixel_data = pixel_data.iter().map(|p| p.to_u8()).flatten().collect::<Vec<_>>();
    img_enc.write_data(&pixel_data).unwrap();
    
    // render_drawing(&mut renderer, canvas.iter().clone())
    
    // Send the buffer to the png file
    // self.writer.write_image_data(&pixel_data).unwrap();
}
