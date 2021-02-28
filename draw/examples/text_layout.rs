use flo_draw::*;
use flo_canvas::*;

use std::sync::*;

///
/// Displays 'Hello, World' in a window
///
pub fn main() {
    with_2d_graphics(|| {
        let lato        = CanvasFontFace::from_slice(include_bytes!("Lato-Regular.ttf"));
        let lato_bold   = CanvasFontFace::from_slice(include_bytes!("Lato-Bold.ttf"));

        // Create a window
        let canvas      = create_canvas_window("Text layout example");

        // Various text layout demonstrations
        canvas.draw(|gc| {
            // Set up the canvas
            gc.canvas_height(1000.0);
            gc.center_region(0.0, 0.0, 1000.0, 1000.0);

            // Load the fonts
            gc.define_font_data(FontId(1), Arc::clone(&lato));
            gc.define_font_data(FontId(2), Arc::clone(&lato_bold));
            gc.set_font_size(FontId(1), 18.0);
            gc.set_font_size(FontId(2), 18.0);

            // Draw some text with layout in these fonts
            gc.fill_color(Color::Rgba(0.0, 0.0, 0.6, 1.0));

            // Start with a simple text layout
            gc.begin_line_layout(18.0, 900.0, TextAlignment::Left);
            gc.layout_text(FontId(1), "Simple text layout".to_string());
            gc.draw_text_layout();

            // We can change fonts during a layout
            gc.begin_line_layout(18.0, 900.0 - 20.0, TextAlignment::Left);
            gc.layout_text(FontId(1), "We can change ".to_string());
            gc.layout_text(FontId(2), "fonts".to_string());
            gc.layout_text(FontId(1), " during layout ".to_string());
            gc.draw_text_layout();

            // We can change colours during a layout
            gc.begin_line_layout(18.0, 900.0 - 40.0, TextAlignment::Left);
            gc.layout_text(FontId(1), "Or we could change ".to_string());
            gc.fill_color(Color::Rgba(0.8, 0.6, 0.0, 1.0));
            gc.layout_text(FontId(1), "colours".to_string());
            gc.fill_color(Color::Rgba(0.0, 0.0, 0.8, 1.0));
            gc.layout_text(FontId(1), " during layout ".to_string());
            gc.draw_text_layout();

            // We can change font sizes during a layout
            gc.begin_line_layout(18.0, 900.0 - 80.0, TextAlignment::Left);
            gc.layout_text(FontId(1), "It's also possible to alter ".to_string());
            gc.set_font_size(FontId(1), 36.0);
            gc.layout_text(FontId(1), "sizes".to_string());
            gc.set_font_size(FontId(1), 18.0);
            gc.layout_text(FontId(1), " during layout ".to_string());
            gc.draw_text_layout();

            // Can align text with all the effects
            gc.begin_line_layout(500.0, 500.0, TextAlignment::Center);
            gc.layout_text(FontId(1), "Text layout demonstration, with changing".to_string());
            gc.set_font_size(FontId(1), 36.0);
            gc.layout_text(FontId(1), " sizes,".to_string());
            gc.set_font_size(FontId(1), 18.0);
            gc.fill_color(Color::Rgba(0.8, 0.6, 0.0, 1.0));
            gc.layout_text(FontId(1), " colours,".to_string());
            gc.fill_color(Color::Rgba(0.0, 0.0, 0.8, 1.0));
            gc.layout_text(FontId(2), " fonts,".to_string());
            gc.layout_text(FontId(1), " and center alignment ".to_string());
            gc.draw_text_layout();
        });
    });
}
