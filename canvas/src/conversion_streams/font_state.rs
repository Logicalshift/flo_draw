use crate::draw::*;
use crate::font_face::*;

// Allsorts is used for shaping, and font-kit for glyph loading and rendering (and finding the font that corresponds to particular properties)
use allsorts;
use allsorts::font::{MatchingPresentation};
use allsorts::gpos;
use allsorts::gsub;
use allsorts::tag;

use ttf_parser::*;

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

impl<'a> OutlineBuilder for FontOutliner<'a> {
    fn move_to(&mut self, x: f32, y: f32) {
        let (x, y)  = (x * self.scale_factor, y * self.scale_factor);

        self.last   = (x, y);

        self.drawing.push(Draw::Move(self.x_pos + x, self.y_pos + y));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let (x, y) = (x * self.scale_factor, y * self.scale_factor);

        self.last   = (x, y);

        self.drawing.push(Draw::Line(self.x_pos + x, self.y_pos + y));
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

        self.drawing.push(Draw::BezierCurve(
            (self.x_pos + x1q, self.y_pos + y1q), 
            (self.x_pos + x2, self.y_pos + y2), 
            (self.x_pos + x3, self.y_pos + y3)));
    }

    fn curve_to(&mut self, cp_x1: f32, cp_y1: f32, cp_x2: f32, cp_y2: f32, to_x: f32, to_y: f32) {
        let (x1, y1)    = (to_x, to_y);
        let (x1, y1)    = (x1 * self.scale_factor, y1 * self.scale_factor);

        let (x2, y2)    = (cp_x1, cp_y1);
        let (x2, y2)    = (x2 * self.scale_factor, y2 * self.scale_factor);
        let (x3, y3)    = (cp_x2, cp_y2);
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
pub (crate) struct FontState {
    /// Fontkit handles for the fonts that are loaded
    loaded_fonts: HashMap<FontId, Arc<CanvasFontFace>>,

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
    /// Clears any loaded fonts
    ///
    pub fn clear(&mut self) {
        self.loaded_fonts   = HashMap::new();
        self.font_size      = HashMap::new();
    }

    ///
    /// Loads a font from a raw data file 
    ///
    pub fn load_font_data(&mut self, id: FontId, data: Arc<CanvasFontFace>) {
        self.loaded_fonts.insert(id, data);
        self.font_size.insert(id, 12.0);
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

        // Load into ttf-parser
        
        // TODO: 'Face' does some parsing so we'd like to not regenerate it every time, 
        // but it also has lifetime requirements that make it impossible to keep around as state
        let font            = font.ttf_font();

        // Fetch some information about this font
        let units_per_em    = font.units_per_em().unwrap_or(16385);
        let scale_factor    = font_size / (units_per_em as f32);
        let mut x_pos       = x;
        let mut y_pos       = y;

        // Produce the drawing for these glyphs
        let mut drawing     = vec![];
        for glyph in glyphs {
            // Fetch information about this glyph
            let glyph_index     = GlyphId(glyph.glyph.glyph_index);
            let advance_x       = font.glyph_hor_advance(glyph_index);
            let advance_y       = font.glyph_ver_advance(glyph_index);
            let advance_x       = if let Some(advance) = advance_x { advance } else { 0 };
            let advance_y       = if let Some(advance) = advance_y { advance } else { 0 };

            // Adjust by any requested offset
            let (off_x, off_y)  = match glyph.placement {
                gpos::Placement::None           => (0.0, 0.0),
                gpos::Placement::Distance(x, y) => (x as f32, y as f32),
                gpos::Placement::Anchor(_ ,_)   => (0.0, 0.0), // TODO: https://docs.microsoft.com/en-us/typography/opentype/spec/gpos#lookup-type-3-cursive-attachment-positioning-subtable
            };
            let off_x           = off_x * scale_factor;
            let off_y           = off_y * scale_factor;

            // Start a new path
            drawing.push(Draw::NewPath);

            // Generate the outline
            let mut outliner    = FontOutliner { 
                drawing: &mut drawing, 
                scale_factor, 
                x_pos: x_pos + off_x, 
                y_pos: y_pos + off_y, 
                last: (0.0, 0.0) 
            };
            font.outline_glyph(glyph_index, &mut outliner);

            // Fill the glyph
            drawing.push(Draw::Fill);

            // Move to the next position
            let advance_x       = (advance_x as f32) + (glyph.kerning as f32);
            let advance_y       = advance_y as f32;
            let advance_x       = advance_x * scale_factor;
            let advance_y       = advance_y * scale_factor;

            x_pos               += advance_x;
            y_pos               += advance_y;
        }

        Some(drawing)
    }
}
