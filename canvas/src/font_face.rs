#[cfg(feature = "outline-fonts")] use allsorts::error::{ParseError};
#[cfg(feature = "outline-fonts")] use allsorts::tables::{FontTableProvider};
#[cfg(feature = "outline-fonts")] use std::borrow::{Cow};

use serde::de::{Deserialize, Deserializer, Visitor, SeqAccess, MapAccess};
use serde::ser::{Serialize, SerializeStruct, Serializer};
use serde::de;

use std::fmt;
use std::sync::*;

/// allsorts table provider implementation based on a unsafe (based on lifetime) pointer to a TTF parser face
#[cfg(feature = "outline-fonts")]
pub struct CanvasTableProvider<'a>(&'a ttf_parser::Face<'a>);

#[cfg(feature = "outline-fonts")] 
impl<'b> FontTableProvider for CanvasTableProvider<'b> {
    fn table_data<'a>(&'a self, tag: u32) -> Result<Option<Cow<'a, [u8]>>, ParseError> {
        let table_data = self.0.table_data(ttf_parser::Tag::from_bytes(&tag.to_be_bytes()));
        let table_data = table_data.map(|data| Cow::Borrowed(data));

        Ok(table_data)
    }

    fn has_table<'a>(&'a self, tag: u32) -> bool {
        let table_data = self.0.table_data(ttf_parser::Tag::from_bytes(&tag.to_be_bytes()));
        table_data.is_some()
    }
}

// Ouroborus doesn't work with #cfg(feature) so we have to duplicate the entire implementation in two modules
#[cfg(not(feature = "outline-fonts"))]
mod canvas_font_face {
    use std::pin::*;
    use std::sync::*;

    ///
    /// Representation of a font face
    ///
    /// This class acquires more features if the `outline-fonts` feature is turned on for
    /// this crate.
    ///
    pub struct CanvasFontFace {
        /// Data for this font face
        data: Arc<Pin<Box<[u8]>>>,
    }

    impl CanvasFontFace {
        #[cfg(not(feature = "outline-fonts"))] 
        #[inline]
        fn borrow_data(&self) -> &Arc<Pin<Box<[u8]>>> {
            &self.data
        }

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
            Arc::new(Self::from_pinned(Arc::new(data.into()), 0))
        }

        pub (crate) fn from_pinned(data: Arc<Pin<Box<[u8]>>>, _font_index: u32) -> CanvasFontFace {
            // Generate the font face
            CanvasFontFace {
                data:       data,
            }
        }

        ///
        /// Retrieves the data bytes for this font
        ///
        pub fn font_data<'a>(&'a self) -> &'a [u8] {
            &***self.borrow_data()
        }
    }
}

#[cfg(feature = "outline-fonts")] 
mod canvas_font_face {
    use super::*;

    use crate::font::*;
    use crate::font_line_layout::*;

    use allsorts;
    use ttf_parser;

    use ouroboros::self_referencing;

    use std::pin::*;

    ///
    /// Representation of a font face
    ///
    /// This class acquires more features if the `outline-fonts` feature is turned on for
    /// this crate.
    ///
    #[self_referencing]
    pub struct CanvasFontFace {
        /// Data for this font face
        data: Arc<Pin<Box<[u8]>>>,

        /// The font face for the data
        #[borrows(data)] #[covariant] ttf_font: ttf_parser::Face<'this>,
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
            Arc::new(Self::from_pinned(Arc::new(data.into()), 0))
        }

        #[cfg(feature = "outline-fonts")]
        pub (crate) fn from_pinned(data: Arc<Pin<Box<[u8]>>>, font_index: u32) -> CanvasFontFace {
            // Load into the TTF parser with scary self-referential data
            let font_face = CanvasFontFaceBuilder {
                data:               data,
                ttf_font_builder:   |data: &Arc<Pin<Box<[u8]>>>| { ttf_parser::Face::from_slice(&**data, font_index as _).unwrap() },
            }.build();

            // Generate the font face
            font_face
        }

        ///
        /// Retrieves the data bytes for this font
        ///
        pub fn font_data<'a>(&'a self) -> &'a [u8] {
            &***self.borrow_data()
        }
    }

    ///
    /// Measures some text in this font
    ///
    #[cfg(feature = "outline-fonts")]
    pub fn measure_text(font: &Arc<CanvasFontFace>, text: &str, em_size: f32) -> TextLayoutMetrics {
        // Create a layout for the text
        let mut layout = CanvasFontLineLayout::new(font, em_size);

        // Layout the text and return the measurements
        layout.add_text(text);
        layout.measure()
    }

    #[cfg(feature = "outline-fonts")]
    impl CanvasFontFace {
        ///
        /// Retrieves the TTF font face for this font
        ///
        pub fn ttf_font<'a>(&'a self) -> &'a ttf_parser::Face<'a> {
            self.borrow_ttf_font()
        }

        ///
        /// Retrieves the base font metrics for this font (None if they can't be determined for this font)
        ///
        pub fn base_font_metrics(&self) -> Option<FontMetrics> {
            let font = self.ttf_font();

            // Result is 'None' if the font has no 'units_per_em' value, as that means we don't know how to scale this font
            Some(FontMetrics {
                em_size:            font.units_per_em() as _,
                ascender:           font.ascender() as _,
                descender:          font.descender() as _,
                height:             font.height() as _,
                line_gap:           font.line_gap() as _,
                capital_height:     font.capital_height().map(|h| h as _),
                underline_position: font.underline_metrics().map(|pos| FontLinePosition { offset: pos.position as _, thickness: pos.thickness as _}),
                strikeout_position: font.strikeout_metrics().map(|pos| FontLinePosition { offset: pos.position as _, thickness: pos.thickness as _}),
            })
        }

        ///
        /// Retrieves the font metrics for this font for a given em-size
        ///
        pub fn font_metrics(&self, em_size: f32) -> Option<FontMetrics> {
            Some(self.base_font_metrics()?.with_size(em_size))
        }
    }

    ///
    /// See `allsorts` for what these functions do
    ///
    #[cfg(feature = "outline-fonts")]
    impl CanvasFontFace {
        ///
        /// Creates a TTF font face for this font
        ///
        pub fn allsorts_font<'a>(&'a self) -> allsorts::Font<CanvasTableProvider<'a>> {
            let face            = self.ttf_font();
            let table_provider  = CanvasTableProvider(face);

            allsorts::Font::new(table_provider)
                .expect("unable to load font tables")
                .expect("unable to find suitable cmap sub-table")
        }
    }
}

pub use self::canvas_font_face::*;

impl PartialEq for CanvasFontFace {
    fn eq(&self, other: &CanvasFontFace) -> bool {
        self.font_data().eq(other.font_data())
    }
}

impl fmt::Debug for CanvasFontFace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CanvasFontFace")
         .field("data", &self.font_data())
         .finish()
    }
}

impl Serialize for CanvasFontFace {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
    S: Serializer {
        let mut s = serializer.serialize_struct("CanvasFontFace", 1)?;
        s.serialize_field("data", self.font_data())?;
        s.end()
    }
}

impl<'de> Deserialize<'de> for CanvasFontFace {
    fn deserialize<D>(deserializer: D) -> Result<CanvasFontFace, D::Error>
    where D: Deserializer<'de> {
        // Field deserializer
        enum Field { Data }
        const FIELDS: &'static [&'static str] = &["data"];

        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
            where D: Deserializer<'de> {
                struct FieldVisitor;
                impl<'de> Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("`data`")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Field, E>
                    where E: de::Error {
                        match value {
                            "data"  => Ok(Field::Data),
                            _       => Err(de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        // Field visitor
        struct CanvasFontFaceVisitor;
        impl<'de> Visitor<'de> for CanvasFontFaceVisitor {
            type Value = CanvasFontFace;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct CanvasFontFace")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<CanvasFontFace, V::Error>
            where V: SeqAccess<'de> {
                let bytes: Vec<u8>  = seq.next_element()? .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let data            = bytes.into_boxed_slice();
                let data            = Arc::new(data.into());
                Ok(CanvasFontFace::from_pinned(data, 0))
            }

            fn visit_map<V>(self, mut map: V) -> Result<CanvasFontFace, V::Error>
            where V: MapAccess<'de> {
                let mut data: Option<Vec<u8>> = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Data => {
                            if data.is_some() {
                                return Err(de::Error::duplicate_field("data"));
                            }
                            data = Some(map.next_value()?);
                        }
                    }
                }

                let data            = data.ok_or_else(|| de::Error::missing_field("data"))?;
                let data            = data.into_boxed_slice();
                let data            = Arc::new(data.into());
                Ok(CanvasFontFace::from_pinned(data, 0))
            }
        }

        // Deserialize the structure
        deserializer.deserialize_struct("CanvasFontFace", FIELDS, CanvasFontFaceVisitor)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_json;

    #[cfg(feature = "outline-fonts")]
    #[test]
    fn load_lato() {
        CanvasFontFace::from_slice(include_bytes!("../test_data/Lato-Regular.ttf"));
    }

    #[cfg(feature = "outline-fonts")]
    #[test]
    fn load_allsorts() {
        let font = CanvasFontFace::from_slice(include_bytes!("../test_data/Lato-Regular.ttf"));
        font.allsorts_font();
    }

    #[test]
    fn serialize_deserialize_font_face() {
        let font    = CanvasFontFace::from_slice(include_bytes!("../test_data/Lato-Regular.ttf"));
        let encoded = serde_json::to_string(&font).unwrap();
        let decoded = serde_json::from_str::<Arc<CanvasFontFace>>(&encoded).unwrap();

        assert!(font == decoded);
    }
}
