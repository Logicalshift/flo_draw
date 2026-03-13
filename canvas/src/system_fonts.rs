use super::font_spec::*;
use super::font_face::*;

use font_kit::handle::{Handle};
use font_kit::source::{SystemSource};

use std::pin::*;
use std::sync::*;

///
/// Loads a system font (with no caching)
///
pub fn load_system_font(spec: impl Into<FontSpec>) -> Option<CanvasFontFace> {
    let spec = spec.into();

    // Use a font-kit system source
    let source = SystemSource::new();

    // Convert the specification into a font-kit specification
    let (family_names, properties) = spec.into();

    // Ask font-kit to retrieve a handle for this font
    let handle = source.select_best_match(&family_names, &properties).ok()?;

    // Load from memory or a file
    let (data, font_index) = match handle {
        Handle::Memory { bytes, font_index } => {
            let boxed: Box<[u8]> = (*bytes).clone().into_boxed_slice();
            (Arc::new(Pin::new(boxed)), font_index)
        }

        Handle::Path { path, font_index } => {
            let bytes = std::fs::read(path).ok()?;
            let boxed: Box<[u8]> = bytes.into_boxed_slice();
            (Arc::new(Pin::new(boxed)), font_index)
        }
    };

    // Convert to a font face
    Some(CanvasFontFace::from_pinned(data, font_index))
}
