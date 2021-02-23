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
use font_kit::outline::{OutlineSink};

use pathfinder_geometry::vector::{Vector2F};
use pathfinder_geometry::line_segment::{LineSegment2F};

use std::sync::*;
use std::collections::{HashMap};

///
/// Structure used to receive outlining instructions from FontKit
///
struct FontOutliner<'a> {
    drawing:        &'a mut Vec<Draw>,
    scale_factor:   f32,
    x_pos:          f32,
    y_pos:          f32,
    last:           (f32, f32)
}

impl<'a> OutlineSink for FontOutliner<'a> {
    fn move_to(&mut self, to: Vector2F) {
        let (x, y)  = (to.x(), to.y());
        let (x, y)  = (x * self.scale_factor, y * self.scale_factor);

        self.last   = (x, y);

        self.drawing.push(Draw::Move(self.x_pos + x, self.y_pos + y));
    }

    fn line_to(&mut self, to: Vector2F) {
        let (x, y) = (to.x(), to.y());
        let (x, y) = (x * self.scale_factor, y * self.scale_factor);

        self.last   = (x, y);

        self.drawing.push(Draw::Line(self.x_pos + x, self.y_pos + y));
    }

    fn quadratic_curve_to(&mut self, ctrl: Vector2F, to: Vector2F) {
        let (x0q, y0q)  = self.last;

        let (x1q, y1q)  = (to.x(), to.y());
        let (x1q, y1q)  = (x1q * self.scale_factor, y1q * self.scale_factor);

        let (x2q, y2q)  = (ctrl.x(), ctrl.y());
        let (x2q, y2q)  = (x2q * self.scale_factor, y2q * self.scale_factor);

        self.last       = (x1q, y1q);

        let (x2, y2)    = (x0q + (2.0/3.0) * (x2q-x0q), y0q + (2.0/3.0) * (y2q-y0q));
        let (x3, y3)    = (x1q + (2.0/3.0) * (x2q-x1q), y1q + (2.0/3.0) * (y2q-y1q));

        self.drawing.push(Draw::BezierCurve(
            (self.x_pos + x1q, self.y_pos + y1q), 
            (self.x_pos + x2, self.y_pos + y2), 
            (self.x_pos + x3, self.y_pos + y3)));
    }

    fn cubic_curve_to(&mut self, ctrl: LineSegment2F, to: Vector2F) {
        let (x1, y1)    = (to.x(), to.y());
        let (x1, y1)    = (x1 * self.scale_factor, y1 * self.scale_factor);

        let (x2, y2)    = (ctrl.from_x(), ctrl.from_y());
        let (x2, y2)    = (x2 * self.scale_factor, y2 * self.scale_factor);
        let (x3, y3)    = (ctrl.to_x(), ctrl.to_y());
        let (x3, y3)    = (x3 * self.scale_factor, y3 * self.scale_factor);

        self.last       = (x1, y1);

        self.drawing.push(Draw::BezierCurve(
            (self.x_pos + x1, self.y_pos + y1), 
            (self.x_pos + x2, self.y_pos + y2), 
            (self.x_pos + x3, self.y_pos + y3)));
    }

    fn close(&mut self) {
        self.drawing.push(Draw::ClosePath);
    }
}

///
/// State of the outline font system (and target for )
///
struct FontState {
    /// Fontkit handles for the fonts that are loaded
    loaded_fonts: HashMap<FontId, Arc<Font>>,

    /// The size specified for each font
    font_size: HashMap<FontId, f32>
}

impl Default for FontState {
    fn default() -> Self {
        FontState {
            loaded_fonts:   HashMap::new(),
            font_size:      HashMap::new()
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
                self.font_size.insert(id, 12.0);
            }

            Err(err) => {
                // Font was not loaded (TODO: some way of handling this error better)
                println!("Could not load font: {:?}", err);
            }
        }
    }

    ///
    /// Updates the size of a particular font
    ///
    pub fn set_font_size(&mut self, id: FontId, new_size: f32) {
        if let Some(size) = self.font_size.get_mut(&id) {
            *size = new_size;
        }
    }

    ///
    /// Retrieves the allsorts font object for a particular font ID
    ///
    /// Returns None if the font is not available or could not be loaded
    ///
    pub fn shape_text<'a>(&'a self, id: FontId, text: String) -> Option<Vec<gpos::Info>> {
        // Fetch the font-kit font
        let font        = if let Some(font) = self.loaded_fonts.get(&id) { font } else { return None; };

        // The font handle contains the font data
        let handle      = if let Some(handle) = font.handle() { handle } else { return None; };

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

    ///
    /// Generates the outlines for some text that has been rendered into glyphs
    ///
    pub fn generate_outlines<'a>(&'a self, id: FontId, glyphs: Vec<gpos::Info>, x: f32, y: f32) -> Option<Vec<Draw>> {
        // Fetch the font-kit font and its size
        let font            = if let Some(font) = self.loaded_fonts.get(&id)    { font } else { return None; };
        let font_size       = if let Some(font_size) = self.font_size.get(&id)  { *font_size } else { return None; };

        // Fetch some information about this font
        let metrics         = font.metrics();
        let scale_factor    = font_size / (metrics.units_per_em as f32);

        // TODO: generate the outlines
        Some(vec![])
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

                Draw::Font(font_id, FontOp::FontSize(new_size)) => {
                    state.set_font_size(font_id, new_size);
                }

                Draw::DrawText(font_id, text, x, y) => {
                    // Call the shaper to generate the glyphs
                    let glyphs = state.shape_text(font_id, text);

                    // Render them as outlines
                    if let Some(glyphs) = glyphs {
                        yield_value(Draw::NewPath).await;

                        for draw in state.generate_outlines(font_id, glyphs, x, y).into_iter().flatten() {
                            yield_value(draw).await;
                        }

                        yield_value(Draw::Fill).await;
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
