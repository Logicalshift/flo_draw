use super::font::*;

#[cfg(feature = "outline-fonts")] use allsorts;
#[cfg(feature = "outline-fonts")] use allsorts::font;
#[cfg(feature = "outline-fonts")] use allsorts::error::{ParseError};
#[cfg(feature = "outline-fonts")] use allsorts::tables::{FontTableProvider};
#[cfg(feature = "outline-fonts")] use ttf_parser;

use std::marker::{PhantomPinned};
use std::slice;
use std::pin::*;
use std::sync::*;
use std::borrow::{Cow};

/// allsorts table provider implementation based on a unsafe (based on lifetime) pointer to a TTF parser face
struct UnsafePinnedTtfTableProvider(*const ttf_parser::Face<'static>);

impl FontTableProvider for UnsafePinnedTtfTableProvider {
    fn table_data<'a>(&'a self, 
        tag: u32) -> Result<Option<Cow<'a, [u8]>>, ParseError> {
        unsafe { 
            let table_data = (*self.0).table_data(ttf_parser::Tag::from_bytes(&tag.to_le_bytes()));
            let table_data = table_data.map(|data| Cow::Borrowed(data));

            Ok(table_data)
        }
    }

    fn has_table<'a>(&'a self, tag: u32) -> bool {
        unsafe { (*self.0).table_data(ttf_parser::Tag::from_bytes(&tag.to_le_bytes())).is_some() }
    }
}

///
/// Representation of a font face
///
/// This class acquires more features if the `outline-fonts` feature is turned on for
/// this crate.
///
pub struct CanvasFontFace {
    /// Data for this font face
    data: Arc<Pin<Box<[u8]>>>,

    /// The font face for the data
    #[cfg(feature = "outline-fonts")] ttf_font: Option<Pin<Box<ttf_parser::Face<'static>>>>,

    /// The allsorts font
    #[cfg(feature = "outline-fonts")] allsorts_font: Option<Mutex<Pin<Box<allsorts::Font<UnsafePinnedTtfTableProvider>>>>>,

    /// The font face is pinned: Allsorts and ttf-parser both need to be able to refer to it
    _pinned: PhantomPinned
}

impl CanvasFontFace {
    ///
    /// Creates a new font by loading the fonts from a slice
    ///
    pub fn from_slice(bytes: &[u8]) -> Arc<CanvasFontFace> {
        Self::from_bytes(Vec::from(bytes))
    }

    ///
    /// Creates a new font by loading the fonts from a byte array
    ///
    pub fn from_bytes(bytes: Vec<u8>) -> Arc<CanvasFontFace> {
        // Pin the data for this font face
        let data = bytes.into_boxed_slice();
        Self::from_pinned(Arc::new(data.into()), 0)
    }

    #[cfg(not(feature = "outline-fonts"))]
    fn from_pinned(data: Arc<Pin<Box<[u8]>>>) -> Arc<CanvasFontFace> {
        // Generate the font face
        Arc::new(CanvasFontFace {
            data:       data,
            _pinned:    PhantomPinned
        })
    }

    #[cfg(feature = "outline-fonts")]
    fn from_pinned(data: Arc<Pin<Box<[u8]>>>, font_index: u32) -> Arc<CanvasFontFace> {
        // Create the data pointer
        let len             = data.len();
        let slice           = data.as_ptr();

        // Load into the TTF parser with scary unsafe self-referential data
        let mut font_face   = CanvasFontFace {
            data:           data,

            ttf_font:       None,
            allsorts_font:  None,
            _pinned:        PhantomPinned
        };

        // TODO: is there a better way? TTF-parser requries a reference to data which either means we need to do this
        // or reload the font every time we use it (which might be OK for large amounts of layout work but probably
        // isn't what we want for reading single glyphs)
        //
        // This 'should' be safe, I think. We've declared the TTF font as 'static but we've pinned it so that it can't
        // be moved away from this structure which manages the lifetime of its owning data. Later on, we force it to be
        // dropped ahead of the data so we're sure that the face no longer exists at the point we drop the data itself.
        //
        // (For allsorts, it seems we can probably implement `FontTableProvider` for an Arc<[u8]> quite easily, but for
        // ttf_parser, it's not really clear how to make a 'static version of FaceTables)
        let ttf_font        = ttf_parser::Face::from_slice(unsafe { slice::from_raw_parts(slice, len) }, font_index as _).unwrap();

        font_face.ttf_font  = Some(Box::pin(ttf_font));

        // Load into allsorts with a similar technique
        let font_ptr        = &**font_face.ttf_font.as_ref().unwrap() as *const _;

        let provider        = UnsafePinnedTtfTableProvider(font_ptr);
        let allsorts_font   = allsorts::Font::new(provider)
            .expect("unable to load font tables")
            .expect("unable to find suitable cmap sub-table");

        font_face.allsorts_font     = Some(Mutex::new(Box::pin(allsorts_font)));

        // Generate the font face
        Arc::new(font_face)
    }

    ///
    /// Retrieves the data bytes for this font
    ///
    pub fn font_data<'a>(&'a self) -> &'a [u8] {
        &**self.data
    }
}

#[cfg(feature = "outline-fonts")]
impl Drop for CanvasFontFace {
    fn drop(&mut self) {
        // Allsorts is using the TTF font so we want to drop that first
        self.allsorts_font  = None;

        // Ensure that the TTF font is dropped before we free the data it's using
        self.ttf_font       = None;

        // Now safe to drop data as nothing is using it
    }
}

#[cfg(feature = "outline-fonts")]
impl CanvasFontFace {
    ///
    /// Retrieves the TTF font face for this font
    ///
    pub fn ttf_font<'a>(&'a self) -> &'a ttf_parser::Face<'a> {
        &**self.ttf_font.as_ref().unwrap()
    }
}

///
/// See `allsorts` for what these functions do
///
#[cfg(feature = "outline-fonts")]
impl CanvasFontFace {
    pub fn num_glyphs(&self) -> u16 { self.allsorts_font.as_ref().unwrap().lock().unwrap().num_glyphs() }

    pub fn lookup_glyph_index(&self, ch: char, match_presentation: font::MatchingPresentation, variation_selector: Option<allsorts::unicode::VariationSelector>) -> (u16, allsorts::unicode::VariationSelector) {
        self.allsorts_font.as_ref().unwrap().lock().unwrap()
            .lookup_glyph_index(ch, match_presentation, variation_selector)
    }

    pub fn shape(&self, glyphs: Vec<allsorts::gsub::RawGlyph<()>>, script_tag: u32, opt_lang_tag: Option<u32>, features: &allsorts::gsub::Features, kerning: bool) -> Result<Vec<allsorts::gpos::Info>,  allsorts::error::ShapingError> {
        self.allsorts_font.as_ref().unwrap().lock().unwrap()
            .shape(glyphs, script_tag, opt_lang_tag, features, kerning)
    }

    pub fn map_glyphs(&mut self, text: &str, match_presentation: font::MatchingPresentation) -> Vec<allsorts::gsub::RawGlyph<()>> {
        self.allsorts_font.as_ref().unwrap().lock().unwrap()
            .map_glyphs(text, match_presentation)
    }
}
