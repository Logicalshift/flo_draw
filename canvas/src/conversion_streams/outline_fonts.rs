use crate::draw::*;
use crate::path::*;
use crate::font::*;
use crate::namespace::*;

use flo_stream::*;

use futures::prelude::*;

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

impl<'a> ttf_parser::OutlineBuilder for FontOutliner<'a> {
    fn move_to(&mut self, x: f32, y: f32) {
        let (x, y)  = (x * self.scale_factor, y * self.scale_factor);

        self.last   = (x, y);

        self.drawing.push(Draw::Path(PathOp::Move(self.x_pos + x, self.y_pos + y)));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let (x, y) = (x * self.scale_factor, y * self.scale_factor);

        self.last   = (x, y);

        self.drawing.push(Draw::Path(PathOp::Line(self.x_pos + x, self.y_pos + y)));
    }

    fn quad_to(&mut self, cp_x1: f32, cp_y1: f32, to_x: f32, to_y:f32) {
        let (x0q, y0q)  = self.last;

        let (x1q, y1q)  = (to_x, to_y);
        let (x1q, y1q)  = (x1q * self.scale_factor, y1q * self.scale_factor);

        let (x2q, y2q)  = (cp_x1, cp_y1);
        let (x2q, y2q)  = (x2q * self.scale_factor, y2q * self.scale_factor);

        self.last       = (x1q, y1q);

        let (x2, y2)    = (x0q + (2.0/3.0) * (x2q-x0q), y0q + (2.0/3.0) * (y2q-y0q));
        let (x3, y3)    = (x1q + (2.0/3.0) * (x2q-x1q), y1q + (2.0/3.0) * (y2q-y1q));

        self.drawing.push(Draw::Path(PathOp::BezierCurve(
            ((self.x_pos + x2, self.y_pos + y2), 
             (self.x_pos + x3, self.y_pos + y3)),
            (self.x_pos + x1q, self.y_pos + y1q), 
        )));
    }

    fn curve_to(&mut self, cp_x1: f32, cp_y1: f32, cp_x2: f32, cp_y2: f32, to_x: f32, to_y: f32) {
        let (x1, y1)    = (to_x, to_y);
        let (x1, y1)    = (x1 * self.scale_factor, y1 * self.scale_factor);

        let (x2, y2)    = (cp_x1, cp_y1);
        let (x2, y2)    = (x2 * self.scale_factor, y2 * self.scale_factor);
        let (x3, y3)    = (cp_x2, cp_y2);
        let (x3, y3)    = (x3 * self.scale_factor, y3 * self.scale_factor);

        self.last       = (x1, y1);

        self.drawing.push(Draw::Path(PathOp::BezierCurve(
            ((self.x_pos + x2, self.y_pos + y2), 
             (self.x_pos + x3, self.y_pos + y3)),
            (self.x_pos + x1, self.y_pos + y1), 
        )));
    }

    fn close(&mut self) {
        self.drawing.push(Draw::Path(PathOp::ClosePath));
    }
}

///
/// Given a stream of drawing instructions (such as is returned by `Canvas::stream()`), turns any glyph drawing instructions 
/// into the equivalent path drawing instructions.
///
/// Along with `drawing_with_laid_out_text`, this can be used to render text to a render target that does not have any font 
/// support of its own.
///
pub fn drawing_with_text_as_paths<InStream>(draw_stream: InStream) -> impl Send+Unpin+Stream<Item=Draw> 
where
    InStream: 'static + Send + Unpin + Stream<Item=Draw>,
{
    generator_stream(move |yield_value| async move {
        // Set up
        let mut namespace_id    = NamespaceId::default().local_id();
        let mut namespace_stack = vec![];
        let mut draw_stream     = draw_stream;
        let mut font_map        = HashMap::new();

        // Pass through the drawing instructions, and process any font instructions that we may come across
        while let Some(draw) = draw_stream.next().await {
            match draw {
                Draw::ClearCanvas(_) => {
                    font_map.clear();
                    namespace_id = NamespaceId::default().local_id();

                    yield_value(draw).await;
                }

                Draw::Namespace(new_namespace) => {
                    namespace_id = new_namespace.local_id();
                }

                Draw::PushState => {
                    namespace_stack.push(namespace_id);
                }

                Draw::PopState => {
                    if let Some(new_namespace) = namespace_stack.pop() {
                        namespace_id = new_namespace;
                    }
                }

                Draw::Font(font_id, FontOp::UseFontDefinition(data)) => {
                    // Store the font to use for this ID
                    font_map.insert((namespace_id, font_id), Arc::clone(&data));
                    yield_value(Draw::Font(font_id, FontOp::UseFontDefinition(data))).await;
                }

                Draw::Font(font_id, FontOp::DrawGlyphs(glyphs)) => {
                    if let Some(font) = font_map.get(&(namespace_id, font_id)) {
                        // Use this font to generate the glyphs
                        let ttf_font        = font.ttf_font();
                        let units_per_em    = ttf_font.units_per_em() as f32;

                        for glyph in glyphs {
                            // Start rendering this glyph
                            yield_value(Draw::Path(PathOp::NewPath)).await;

                            let GlyphId(glyph_id)   = glyph.id;
                            let glyph_id            = ttf_parser::GlyphId(glyph_id as _);

                            // Generate the outline
                            let mut drawing         = vec![];
                            let mut outliner        = FontOutliner { 
                                drawing:        &mut drawing,
                                scale_factor:   glyph.em_size / units_per_em,
                                x_pos:          glyph.location.0,
                                y_pos:          glyph.location.1,
                                last:           (0.0, 0.0)
                            };

                            ttf_font.outline_glyph(glyph_id, &mut outliner);

                            // Render the drawing
                            for draw in drawing {
                                yield_value(draw).await;
                            }

                            // Fill the path
                            yield_value(Draw::Fill).await;
                        }
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
    use super::super::glyph_layout::*;
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
            let instructions    = drawing_with_laid_out_text(instructions);
            let instructions    = drawing_with_text_as_paths(instructions);

            let _instructions   = instructions.collect::<Vec<_>>().await;
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
            let instructions    = drawing_with_laid_out_text(instructions);
            let instructions    = drawing_with_text_as_paths(instructions);

            let instructions    = instructions.collect::<Vec<_>>().await;

            // The font stream should generate some glyph rendering
            println!("{:?}", instructions);
            assert!(instructions.len() != 0);
        });
    }
}
