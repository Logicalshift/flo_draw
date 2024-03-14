use flo_render_software::draw::*;
use flo_render_software::pixel::*;
use flo_render_software::render::*;
use flo_render_software::scanplan::*;

use flo_render_software::canvas::*;

use futures::prelude::*;
use futures::stream;
use futures::executor;

use std::sync::*;
use std::time::{Instant};

///
/// Render some text using the canvas's text-to-outline converer from flo_canvas
///
pub fn main() {
    // Load the Lato font that we'll use for the test
    let lato        = CanvasFontFace::from_slice(include_bytes!("../test_data/Lato-Regular.ttf"));
    let lato_bold   = CanvasFontFace::from_slice(include_bytes!("../test_data/Lato-Bold.ttf"));

    // Create some drawing commands to render the text
    let mut gc = vec![];

    gc.clear_canvas(Color::Rgba(1.0, 1.0, 1.0, 1.0));
    gc.canvas_height(1080.0);
    gc.center_region(0.0, 0.0, 1920.0, 1080.0);

    gc.define_font_data(FontId(0), Arc::clone(&lato));
    gc.define_font_data(FontId(1), Arc::clone(&lato_bold));

    // Lay out some text in the graphics context
    gc.set_font_size(FontId(1), 48.0);
    gc.draw_text(FontId(1), "Rendering text with the software renderer".to_string(), 64.0, 1080.0 - 48.0 - 64.0);
    gc.set_font_size(FontId(0), 16.0);

    gc.draw_text(FontId(0), "This is performing text rendering by first converting this text to paths. This is something of a torture test for the software renderer for a few reasons:".to_string(), 64.0, 1080.0 - 48.0*2.0 - 64.0);
    gc.draw_text(FontId(0), "• There's no hinting for these paths and they contain fine detail".to_string(), 64.0 + 48.0, 1080.0 - 48.0*2.0-18.0*1.0 - 64.0);
    gc.draw_text(FontId(0), "• This generates a large number of fairly complicated paths to render".to_string(), 64.0 + 48.0, 1080.0 - 48.0*2.0-18.0*2.0 - 64.0);
    gc.draw_text(FontId(0), "• 'Good' font rendering is a hugely subjective thing with arguments about what makes something 'crisp' or otherwise".to_string(), 64.0 + 48.0, 1080.0 - 48.0*2.0-18.0*3.0 - 64.0);
    gc.draw_text(FontId(0), "• 'Good' font rendering is also pretty objective with things like vertical spacing and kerning to consider".to_string(), 64.0 + 48.0, 1080.0 - 48.0*2.0-18.0*4.0 - 64.0);
    gc.draw_text(FontId(0), "• The standard 'shard' scan planner only considers anti-aliasing in the horizontal plane, which doesn't work well for fonts with thin horizontal lines".to_string(), 64.0 + 48.0, 1080.0 - 48.0*2.0-18.0*5.0 - 64.0);

    // Convert the font instructions to 'normal' drawing instructions (bypassing any renderer that might be added by the software renderer)
    let drawing = stream::iter(gc);
    let drawing = drawing_with_laid_out_text(drawing);
    let drawing = drawing_with_text_as_paths(drawing);
    let drawing = executor::block_on(async move { drawing.collect::<Vec<_>>().await });

    // Time how long it takes to draw the text to the canvas
    for _ in 0..10 {
        let mut canvas_drawing = CanvasDrawing::<F32LinearPixel, 4>::empty();
        canvas_drawing.draw(drawing.iter().cloned());
    }

    let render_start = Instant::now();
    for _ in 0..100 {
        let mut canvas_drawing = CanvasDrawing::<F32LinearPixel, 4>::empty();
        canvas_drawing.draw(drawing.iter().cloned());
    }
    let render_time = Instant::now().duration_since(render_start);
    let avg_micros  = render_time.as_micros() / 100;
    println!("Canvas drawing time: {}.{}ms", avg_micros/1000, avg_micros%1000);

    // Time some rendering (useful for profiling/optimisation)
    let mut canvas_drawing  = CanvasDrawing::<F32LinearPixel, 4>::empty();
    let mut frame           = vec![0u8; 1920*1080*4];
    let mut rgba            = RgbaFrame::from_bytes(1920, 1080, 2.2, &mut frame).unwrap();

    canvas_drawing.draw(drawing.iter().cloned());

    // Warm up before timing the rendering
    for _ in 0..10 {
        let renderer = CanvasDrawingRegionRenderer::new(ShardScanPlanner::default(), ScanlineRenderer::new(canvas_drawing.program_runner(1080.0)), 1080);
        rgba.render(renderer, &canvas_drawing);
    }

    let render_start = Instant::now();
    for _ in 0..100 {
        let renderer = CanvasDrawingRegionRenderer::new(ShardScanPlanner::default(), ScanlineRenderer::new(canvas_drawing.program_runner(1080.0)), 1080);
        rgba.render(renderer, &canvas_drawing);
    }
    let render_time = Instant::now().duration_since(render_start);
    let avg_micros  = render_time.as_micros() / 100;
    println!("F32 frame render time: {}.{}ms", avg_micros/1000, avg_micros%1000);

    // Render to the terminal window
    render_drawing(&mut TerminalRenderTarget::new(1920, 1080), drawing.iter().cloned());
}