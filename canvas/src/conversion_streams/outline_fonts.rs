use super::font_state::*;

use crate::draw::*;
use crate::font::*;

use flo_stream::*;

use futures::prelude::*;

///
/// Given a stream of drawing instructions (such as is returned by `Canvas::stream()`), processes any font or text instructions
/// so that they are removed and replaced with path instructions
///
/// This can be used to render text to a render target that does not have any font support of its own.
///
pub fn drawing_with_text_as_paths<InStream: 'static+Send+Unpin+Stream<Item=Draw>>(draw_stream: InStream) -> impl Send+Unpin+Stream<Item=Draw> {
    generator_stream(move |yield_value| async move {
        // Set up
        let mut draw_stream = draw_stream;
        let mut state       = FontState::default();

        // Pass through the drawing instructions, and process any font instructions that we may come across
        while let Some(draw) = draw_stream.next().await {
            match draw {
                Draw::ClearCanvas(_) => {
                    state.clear();

                    yield_value(draw).await;
                }

                Draw::Font(font_id, FontOp::UseFontDefinition(data)) => {
                    state.load_font_data(font_id, data);
                }

                Draw::Font(font_id, FontOp::FontSize(new_size)) => {
                    state.set_font_size(font_id, new_size);
                }

                Draw::DrawText(font_id, text, x, y) => {
                    // Call the shaper to generate the glyphs
                    let glyphs = state.shape_text(font_id, text);

                    // Render them as outlines
                    if let Some(glyphs) = glyphs {
                        for draw in state.generate_outlines(font_id, glyphs, x, y).into_iter().flatten() {
                            yield_value(draw).await;
                        }

                        yield_value(Draw::NewPath).await;
                    }
                }

                _ => {
                    // Send non-text instructions through as-is
                    yield_value(draw).await;
                }
            }
        }
    })
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::font_face::*;
    use futures::stream;
    use futures::executor;

    #[test]
    fn load_font_from_bytes() {
        executor::block_on(async {
            // Set up loading a font from a byte stream
            let lato            = CanvasFontFace::from_slice(include_bytes!("../../test_data/Lato-Regular.ttf"));

            let instructions    = vec![Draw::Font(FontId(1), FontOp::UseFontDefinition(lato))];
            let instructions    = stream::iter(instructions);
            let instructions    = drawing_with_text_as_paths(instructions);

            let instructions    = instructions.collect::<Vec<_>>().await;

            // The font stream should consume the load instruction
            assert!(instructions.len() == 0);
        });
    }

    #[test]
    fn draw_text() {
        executor::block_on(async {
            // Set up loading a font from a byte stream
            let lato            = CanvasFontFace::from_slice(include_bytes!("../../test_data/Lato-Regular.ttf"));

            let instructions    = vec![
                Draw::Font(FontId(1), FontOp::UseFontDefinition(lato)), 
                Draw::Font(FontId(1), FontOp::FontSize(12.0)),
                Draw::DrawText(FontId(1), "Hello".to_string(), 100.0, 200.0),
            ];
            let instructions    = stream::iter(instructions);
            let instructions    = drawing_with_text_as_paths(instructions);

            let instructions    = instructions.collect::<Vec<_>>().await;

            // The font stream should generate some glyph rendering
            println!("{:?}", instructions);
            assert!(instructions.len() != 0);
        });
    }
}
