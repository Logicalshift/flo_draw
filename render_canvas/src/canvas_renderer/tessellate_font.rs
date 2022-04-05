use super::canvas_renderer::*;

use flo_canvas as canvas;

// The font routines are expected to be implemented by post-processing the output stream of rendering instructions, so they are currently empty here
// See `drawing_with_laid_out_text()` and ` drawing_with_text_as_paths` from flo_canvas for one way to achieve this

impl CanvasRenderer {
    ///
    /// Performs an operation on a font
    ///
    #[inline]
    pub (super) fn tes_font(&mut self, _font_id: canvas::FontId, _font_op: canvas::FontOp) { }

    ///
    /// Begins laying out text on a line: the coordinates specify the baseline position
    ///
    #[inline]
    pub (super) fn tes_begin_line_layout(&mut self, _x: f32, _y: f32, _aligment: canvas::TextAlignment) { }

    ///
    /// Renders the text in the current layout
    ///
    #[inline]
    pub (super) fn tes_draw_laid_out_text(&mut self) { }

    ///
    /// Draws a string using a font with a baseline starting at the specified position
    ///
    #[inline]
    pub (super) fn tes_draw_text(&mut self, _font_id: canvas::FontId, _text: String, _x: f32, _y: f32) { }
}
