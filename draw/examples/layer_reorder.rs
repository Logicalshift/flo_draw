use flo_draw::*;
use flo_canvas::*;

use std::thread;
use std::time::{Duration};

use rand::*;

pub fn main() {
    with_2d_graphics(|| {
        // Create a window with a canvas to draw on
        let canvas      = create_drawing_window("Reordering layers");
        let lato_bold   = CanvasFontFace::from_slice(include_bytes!("Lato-Bold.ttf"));

        canvas.draw(|gc| {
            gc.clear_canvas(Color::Rgba(0.5, 0.8, 1.0, 1.0));
            gc.canvas_height(1000.0);
            gc.center_region(0.0, 0.0, 1000.0, 1000.0);

            gc.define_font_data(FontId(1), lato_bold);
            gc.set_font_size(FontId(1), 1000.0);

            gc.layer(LayerId(0));
            gc.fill_color(Color::Rgba(0.0, 0.0, 0.6, 1.0));

            gc.begin_line_layout(500.0, 150.0, TextAlignment::Center);
            gc.layout_text(FontId(1), "A".to_string());
            gc.draw_text_layout();

            gc.layer(LayerId(1));
            gc.fill_color(Color::Rgba(0.0, 0.6, 0.0, 1.0));

            gc.begin_line_layout(500.0, 150.0, TextAlignment::Center);
            gc.layout_text(FontId(1), "B".to_string());
            gc.draw_text_layout();

            gc.layer(LayerId(2));
            gc.fill_color(Color::Rgba(0.8, 0.8, 0.0, 1.0));

            gc.begin_line_layout(500.0, 150.0, TextAlignment::Center);
            gc.layout_text(FontId(1), "C".to_string());
            gc.draw_text_layout();
        });

        loop {
            thread::sleep(Duration::from_secs(2));

            let first_layer     = random_range(0..3);
            let second_layer    = random_range(0..3);

            println!("{:?} before {:?}", second_layer, first_layer);

            canvas.draw(|gc| {
                gc.layer(LayerId(first_layer));
                gc.place_layer_before(NamespaceId::default(), LayerId(second_layer));
            })
        }
    });
}
