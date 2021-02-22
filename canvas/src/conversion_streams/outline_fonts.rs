use crate::draw::*;
use crate::font::*;

use flo_stream::*;

use futures::prelude::*;

// Allsorts is used for shaping, and font-kit for glyph loading and rendering (and finding the font that corresponds to particular properties)
use allsorts;
use font_kit::handle::{Handle};
use font_kit::loaders::default::{Font};

use std::sync::*;
use std::collections::{HashMap};

///
/// State of the outline font system (and target for )
///
struct FontState {
    /// Fontkit handles for the fonts that are loaded
    loaded_fonts: HashMap<FontId, Arc<Font>>
}

impl Default for FontState {
    fn default() -> Self {
        FontState {
            loaded_fonts: HashMap::new()
        }
    }
}
impl FontState {
    ///
    /// Loads a font from a raw data file 
    ///
    pub fn load_font_data(&mut self, id: FontId, data: Arc<Vec<u8>>) {
        match Font::from_bytes(data, 0) {
            Ok(font) => {
                self.loaded_fonts.insert(id, Arc::new(font));
            }

            Err(err) => {
                println!("Could not load font: {:?}", err);
            }
        }
    }
}


///
/// Given a stream of drawing instructions (such as is returned by `Canvas::stream()`), processes any font or text instructions
/// so that they are removed and replaced with path instructions
///
/// This can be used to render text to a render target that does not have any font support of its own.
///
pub fn stream_outline_fonts<InStream: 'static+Send+Unpin+Stream<Item=Draw>>(draw_stream: InStream) -> impl Send+Unpin+Stream<Item=Draw> {
    generator_stream(move |yield_value| async move {
        // Set up
        let mut draw_stream = draw_stream;
        let mut state       = FontState::default();

        // Pass through the drawing instructions, and process any font instructions that we may come across
        while let Some(draw) = draw_stream.next().await {
            match draw {
                Draw::Font(font_id, FontOp::UseFontDefinition(FontData::Ttf(data))) |
                Draw::Font(font_id, FontOp::UseFontDefinition(FontData::Otf(data))) => {
                    state.load_font_data(font_id, data);
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
    use futures::stream;
    use futures::executor;

    #[test]
    fn load_font_from_bytes() {
        executor::block_on(async {
            // Set up loading a font from a byte stream
            let lato            = Arc::new(Vec::from(include_bytes!("../../test_data/Lato-Regular.ttf").clone()));

            let instructions    = vec![Draw::Font(FontId(1), FontOp::UseFontDefinition(FontData::Ttf(lato)))];
            let instructions    = stream::iter(instructions);
            let instructions    = stream_outline_fonts(instructions);

            let instructions    = instructions.collect::<Vec<_>>().await;

            // The font stream should consume the load instruction
            assert!(instructions.len() == 0);
        });
    }
}
