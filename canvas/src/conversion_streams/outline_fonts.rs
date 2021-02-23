use crate::draw::*;
use crate::font::*;

use flo_stream::*;

use futures::prelude::*;

// Allsorts is used for shaping, and font-kit for glyph loading and rendering (and finding the font that corresponds to particular properties)
use allsorts;
use allsorts::binary::read::{ReadScope};
use allsorts::font::{MatchingPresentation};
use allsorts::font_data;
use allsorts::font_data::{DynamicFontTableProvider};
use allsorts::gpos;
use allsorts::gsub;
use allsorts::tag;

use font_kit::handle::{Handle};
use font_kit::loaders::default::{Font};

use std::sync::*;
use std::collections::{HashMap};

///
/// State of the outline font system (and target for )
///
struct FontState {
    /// Fontkit handles for the fonts that are loaded
    loaded_fonts: HashMap<FontId, Arc<Font>>,
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
                // Font was successfully loaded: add to the loaded-fonts list
                self.loaded_fonts.insert(id, Arc::new(font));
            }

            Err(err) => {
                // Font was not loaded (TODO: some way of handling this error better)
                println!("Could not load font: {:?}", err);
            }
        }
    }

    ///
    /// Retrieves the allsorts font object for a particular font ID
    ///
    /// Returns None if the font is not available or could not be loaded
    ///
    pub fn shape_text<'a>(&'a self, id: FontId, text: String) -> Option<Vec<gpos::Info>> {
        // Fetch the font-kit font
        let font    = if let Some(font) = self.loaded_fonts.get(&id) { font } else { return None; };

        // The font handle contains the font data
        let handle  = if let Some(handle) = font.handle() { handle } else { return None; };

        // Retrieve the font data
        let (data, font_index) = match handle {
            Handle::Path { .. }                     => { return None; /* TODO */}
            Handle::Memory { bytes, font_index }    => { (bytes, font_index) }
        };

        let scope       = ReadScope::new(&*data);
        let font_file   = scope.read::<font_data::FontData<'_>>().ok()?;
        let provider    = font_file.table_provider(font_index as _).ok()?;
        let mut font    = allsorts::Font::new(provider)
            .expect("unable to load font tables")
            .expect("unable to find suitable cmap sub-table");

        // Map glyphs
        let glyphs      = font.map_glyphs(&text, MatchingPresentation::NotRequired);

        // Shape
        let shape       = font.shape(glyphs, tag::LATN, Some(tag::DFLT), &gsub::Features::Mask(gsub::GsubFeatureMask::default()), true).ok()?;

        Some(shape)
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

                Draw::DrawText(font_id, text, x, y) => {
                    let shape = state.shape_text(font_id, text);
                    if let Some(shape) = shape {
                        println!("OK {:?}", shape);
                    } else {
                        println!("Bleh");
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

    #[test]
    fn draw_text() {
        executor::block_on(async {
            // Set up loading a font from a byte stream
            let lato            = Arc::new(Vec::from(include_bytes!("../../test_data/Lato-Regular.ttf").clone()));

            let instructions    = vec![
                Draw::Font(FontId(1), FontOp::UseFontDefinition(FontData::Ttf(lato))), 
                Draw::Font(FontId(1), FontOp::FontSize(12.0)),
                Draw::DrawText(FontId(1), "Hello".to_string(), 100.0, 200.0),
            ];
            let instructions    = stream::iter(instructions);
            let instructions    = stream_outline_fonts(instructions);

            let instructions    = instructions.collect::<Vec<_>>().await;

            // The font stream should generate some glyph rendering
            println!("{:?}", instructions);
            assert!(instructions.len() != 0);
            assert!(false);
        });
    }
}
