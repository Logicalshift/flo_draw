use flo_draw::*;
use flo_draw::canvas::*;

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
            gc.begin_line_layout(18.0, 900.0 - 30.0, TextAlignment::Left);
            gc.layout_text(FontId(1), "We can change ".to_string());
            gc.layout_text(FontId(2), "fonts".to_string());
            gc.layout_text(FontId(1), " during layout ".to_string());
            gc.draw_text_layout();

            // We can change colours during a layout
            gc.begin_line_layout(18.0, 900.0 - 60.0, TextAlignment::Left);
            gc.layout_text(FontId(1), "Or we could change ".to_string());
            gc.fill_color(Color::Rgba(0.8, 0.6, 0.0, 1.0));
            gc.layout_text(FontId(1), "colours".to_string());
            gc.fill_color(Color::Rgba(0.0, 0.0, 0.6, 1.0));
            gc.layout_text(FontId(1), " during layout ".to_string());
            gc.draw_text_layout();

            // We can change font sizes during a layout
            gc.begin_line_layout(18.0, 900.0 - 100.0, TextAlignment::Left);
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
            gc.fill_color(Color::Rgba(0.0, 0.0, 0.6, 1.0));
            gc.layout_text(FontId(2), " fonts,".to_string());
            gc.layout_text(FontId(1), " and center alignment ".to_string());
            gc.draw_text_layout();

            gc.begin_line_layout(1000.0-18.0, 80.0, TextAlignment::Right);
            gc.layout_text(FontId(1), "Right alignment is supported too".to_string());
            gc.draw_text_layout();

            // Can perform fully manual layout, and annotate with other drawing
            let mut text_layout = CanvasFontLineLayout::new(&lato, 18.0);
            text_layout.layout_text("Performing layout manually is also possible");

            // Calling 'align_transform' moves the text to its final position, and 'to_drawing' generates the drawing instructions for the layout (the layout needs to know the FontId to generate drawing instructions)
            text_layout.align_transform(500.0, 400.0, TextAlignment::Center);
            gc.draw_list(text_layout.to_drawing(FontId(1)));

            // We can use the measure() and draw() functions to add annotations to the text as we generate the layout
            // font_metrics(em_size) gives some information about a particular font
            let lato_metrics = lato.font_metrics(18.0).unwrap();
            let mut text_layout = CanvasFontLineLayout::new(&lato, 18.0);
            
            text_layout.layout_text("Manual layout allows ");

            // 'measure()' interrupts the layout, so measuring half-way between 'f' and 'i' will force the layout to produce no ligature
            let start_pos   = text_layout.measure();
            text_layout.layout_text("custom");
            let end_pos     = text_layout.measure();

            // start_pos and end_pos show where the word 'custom' began and ended (as well as giving the overall bounding box)
            // This is a fairly simple drawing that just renders an underline in a different colour (note that Vec<Draw> implements GraphicsContext, so we can use it to buffer the drawing operations we want to perform)
            let mut underline = vec![];
            underline.new_path();
            underline.move_to(start_pos.pos.x() as _, start_pos.pos.y() as f32 + lato_metrics.underline_position.unwrap().offset);
            underline.line_to(end_pos.pos.x() as _, end_pos.pos.y() as f32 + lato_metrics.underline_position.unwrap().offset);
            underline.stroke_color(Color::Rgba(0.8, 0.6, 0.0, 1.0));
            underline.line_width(lato_metrics.underline_position.unwrap().thickness);
            underline.stroke();

            // Add the underling drawing we constructed to the text layout
            text_layout.draw(underline);

            // Finish up the text...
            text_layout.layout_text(" drawing effects, such as this underline");

            // ... and align it using align_transform so the underline is moved along with the text
            text_layout.align_transform(500.0, 370.0, TextAlignment::Center);
            gc.draw_list(text_layout.to_drawing(FontId(1)));

            // It's still possible to change fonts and colours while using a manual layout
            let mut text_layout = CanvasFontLineLayout::new(&lato, 18.0);
            text_layout.layout_text("Changing ");
            text_layout.draw(vec![Draw::FillColor(Color::Rgba(0.8, 0.6, 0.0, 1.0))]);
            text_layout.layout_text("colour");
            text_layout.draw(vec![Draw::FillColor(Color::Rgba(0.0, 0.0, 0.6, 1.0))]);
            text_layout.layout_text(" and ");

            // FontId 1 = lato, FontId 2 = lato bold (note we supply the old font ID and not the new one here!)
            let mut text_layout = text_layout.continue_with_new_font(FontId(1), &lato_bold, 18.0);
            text_layout.layout_text("font");
            let mut text_layout = text_layout.continue_with_new_font(FontId(2), &lato, 18.0);
            text_layout.layout_text(" is still possible with manual layouts");

            // Calling 'align_transform' moves the text to its final position, and 'to_drawing' generates the drawing instructions for the layout (the layout needs to know the FontId to generate drawing instructions)
            text_layout.align_transform(500.0, 340.0, TextAlignment::Center);
            gc.draw_list(text_layout.to_drawing(FontId(1)));
        });
    });
}
