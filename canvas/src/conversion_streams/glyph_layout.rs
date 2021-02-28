use crate::draw::*;
use crate::font::*;
use crate::font_line_layout::*;

use flo_stream::*;

use futures::prelude::*;

use std::mem;
use std::iter;
use std::sync::*;
use std::collections::{HashMap};

///
/// Given a stream with font instructions, replaces any layout instruction (eg, `Draw::DrawText()`) with glyph
/// rendering instructions
///
pub fn drawing_with_laid_out_text<InStream: 'static+Send+Unpin+Stream<Item=Draw>>(draw_stream: InStream) -> impl Send+Unpin+Stream<Item=Draw> {
    generator_stream(move |yield_value| async move {
        let mut draw_stream         = draw_stream;

        // State of this stream
        let mut font_map            = HashMap::new();
        let mut font_size           = HashMap::new();
        let mut current_line        = None;
        let mut current_font        = None;
        let (mut x_pos, mut y_pos)  = (0.0, 0.0);
        let mut alignment           = TextAlignment::Left;

        // Read from the drawing stream
        while let Some(draw) = draw_stream.next().await {
            match draw {
                Draw::Font(font_id, FontOp::UseFontDefinition(font_defn)) => {
                    // Store this font definition
                    font_map.insert(font_id, Arc::clone(&font_defn));
                    font_size.insert(font_id, 12.0);

                    // Send the font to the next part of the stream
                    yield_value(Draw::Font(font_id, FontOp::UseFontDefinition(font_defn))).await;
                }

                Draw::BeginLineLayout(x, y, align)   => {
                    // If we're laying out text already, this discards that layout
                    current_line    = None;
                    current_font    = None;

                    // Set up the layout for the next set of text
                    x_pos           = x;
                    y_pos           = y;
                    alignment       = align;
                }

                Draw::Font(font_id, FontOp::LayoutText(text)) => {
                    // Update the current font
                    if current_font != Some(font_id) {
                        if let (Some(new_font), Some(font_size)) = (font_map.get(&font_id), font_size.get(&font_id)) {
                            let last_font   = current_font.unwrap_or(FontId(0));
                            let new_font    = Arc::clone(new_font);
                            let font_size   = *font_size;

                            current_line = current_line
                                .map(|line: CanvasFontLineLayout| {
                                    line.continue_with_new_font(last_font, &new_font, font_size)
                                }).or_else(|| {
                                    Some(CanvasFontLineLayout::new(&new_font, font_size))
                                });
                            current_font = Some(font_id);
                        }
                    }

                    // Lay out the text
                    current_line.as_mut().map(|line| line.layout_text(&text));
                }

                Draw::DrawLaidOutText => {
                    if let Some(layout) = mem::take(&mut current_line) {
                        // Align the layout
                        let mut layout = layout;
                        layout.align(x_pos, y_pos, alignment);

                        if let Some(current_font) = mem::take(&mut current_font) {
                            // Convert to drawing actions, and send those
                            let drawing = layout.to_drawing(current_font);

                            for draw in drawing {
                                yield_value(draw).await;
                            }
                        }
                    }
                },

                Draw::FillColor(fill_color) => {
                    // This is added as a drawing instruction to the current layout
                    if let Some(current_line) = &mut current_line {
                        current_line.draw(iter::once(Draw::FillColor(fill_color.clone())));
                    }

                    yield_value(Draw::FillColor(fill_color)).await;
                },

                Draw::ClearCanvas(_) => {
                    // Clear state
                    font_map        = HashMap::new();
                    current_line    = None;

                    yield_value(draw).await;
                }

                // Default action is just to pass the drawing on
                _ => {
                    yield_value(draw).await;
                }
            }
        }
    })
}
