use super::font::*;

///
/// The type of font to look up
///
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub enum FontFamily {
    SystemUI,
    Serif,
    SansSerif,
    Monospace,
    Cursive,
    Fantasy,
}

///
/// Describes the specification of a font
///
#[derive(Clone, Debug, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct FontSpec {
    /// The named fonts to use
    family_names: Vec<String>,

    /// The family of fonts to use
    family: Option<FontFamily>,

    /// The requested style
    style: Option<FontStyle>,

    /// The weight of the font to choose (100-900), 400 being the 'normal' weight and 900 being superbold
    weight: Option<u32>,
}

impl Default for FontSpec {
    ///
    /// Returns a FontSpec that represents the default system font for the system that we're running on
    ///
    fn default() -> Self {
        FontSpec {
            family_names:   vec![],
            family:         None,
            style:          None,
            weight:         None,
        }
    }

}

impl FontSpec {
    ///
    /// Adjusts this specification with a family name
    ///
    pub fn with_family_name(mut self, name: impl Into<String>) -> Self {
        self.family_names = vec![name.into()];

        self
    }

    ///
    /// Specifies an alternative family name that will be used if the main one isn't available
    ///
    /// Any number of alternative names can be specified
    ///
    pub fn with_alternative_family_name(mut self, name: impl Into<String>) -> Self {
        self.family_names.push(name.into());

        self
    }

    ///
    /// Adds a suffix to the requested family names
    ///
    #[inline] pub fn with_family_name_suffix(mut self, suffix: impl Into<String>) -> Self {
        let suffix = suffix.into();

        self.family_names.iter_mut()
            .for_each(|name| *name += &suffix);
        
        self
    }

    ///
    /// Sets the family to use for this font (if the name is not specified or not available)
    ///
    /// This can be used to pick a generic font of a particular type
    ///
    pub fn with_family(mut self, family: FontFamily) -> Self {
        self.family = Some(family);

        self
    }

    ///
    /// Sets the style of the font (whether or not it's italic or oblique)
    ///
    pub fn with_style(mut self, style: FontStyle) -> Self {
        self.style = Some(style);

        self
    }

    /// Uses the lightest possible font weight
    pub fn with_thin_weight(mut self) -> Self { self.weight = Some(100); self }

    /// Uses an extra light font weight
    pub fn with_extra_light_weight(mut self) -> Self { self.weight = Some(200); self }

    /// Uses a light font weight
    pub fn with_light_weight(mut self) -> Self { self.weight = Some(300); self }

    /// Uses the normal font weight
    pub fn with_normal_weight(mut self) -> Self { self.weight = Some(400); self }

    /// Uses the medium font weight (slightly heavier than normal)
    pub fn with_medium_weight(mut self) -> Self { self.weight = Some(500); self }

    /// Uses the semibold weight
    pub fn with_semibold_weight(mut self) -> Self { self.weight = Some(600); self }

    /// Use the standard bold weight
    pub fn with_bold_weight(mut self) -> Self { self.weight = Some(700); self }

    /// Uses the extra bold weight
    pub fn with_extra_bold_weight(mut self) -> Self { self.weight = Some(800); self }

    /// Uses the black weight (heaviest possible value)
    pub fn with_black_weight(mut self) -> Self { self.weight = Some(900); self }

    /// Uses a specific weight value (100-900) 
    pub fn with_weight(mut self, weight: u32) -> Self { self.weight = Some(weight); self }

    /// Returns the list of family names for this font specification
    #[inline] pub fn family_names(&self) -> &[String] { &self.family_names }

    /// Returns the font family for this font specification, if one is set
    #[inline] pub fn family(&self) -> Option<FontFamily> { self.family }

    /// Returns the font style for this font specification, if one is set
    #[inline] pub fn style(&self) -> Option<FontStyle> { self.style }

    /// Returns the font weight for this font specification, if one is set
    #[inline] pub fn weight(&self) -> Option<u32> { self.weight }

    ///
    /// Returns possible suffixes for the weight class of this font, in lightly order of how appropriate they are
    ///
    /// This is to work around a bug in font-kit on Mac OS where it doesn't look up the fonts with the  weight properties
    /// set correctly.
    ///
    pub fn weight_suffix(&self) -> Vec<&'static str> {
        let Some(weight) = self.weight else { return vec!["", " Regular", " Normal", " Medium"] };

        if weight < 200 {
            vec![" Thin", " UltraLight"]
        } else if weight < 300 {
            vec![" ExtraLight", " UltraLight", " Thin"]
        } else if weight < 400 {
            vec![" Light", " ExtraLight", " Thin"]
        } else if weight < 500 {
            vec![" Regular", " Normal", " Medium"]
        } else if weight < 600 {
            vec![" Medium", " Normal"]
        } else if weight < 700 {
            vec![" SemiBold", " Bold", " Medium"]
        } else if weight < 800 {
            vec![" Bold", " SemiBold", " Medium"]
        } else if weight < 900 {
            vec![" ExtraBold", " Bold", " SemiBold", " Medium"]
        } else {
            vec![" Black", " UltraBold", " ExtraBold", " Bold", " SemiBold", " Medium"]
        }
    }

    ///
    /// Returns a 'score' for how close this spec is to another spec
    ///
    /// 'self' here is the spec we're trying to match against, and 'other' is the candidate spec we want to
    /// see how well it fits
    ///
    pub fn score(&self, other: &FontSpec) -> f64 {
        // Difference in weights
        let weight_diff = if let (Some(our_weight), Some(their_weight)) = (self.weight, other.weight) {
            (our_weight as f64 - their_weight as f64).abs()
        } else {
            0.0
        };

        // Difference in styles
        let style_diff = if let (Some(our_style), Some(their_style)) = (self.style, other.style) {
            if our_style == their_style {
                0.0
            } else {
                200.0
            }
        } else {
            0.0
        };

        // Difference in families
        let family_diff = if let (Some(our_family), Some(their_familiy)) = (self.family, other.family) {
            if our_family == their_familiy {
                0.0
            } else {
                5000.0
            }
        } else { 
            0.0
        };

        // Difference in names
        // We're forgiving of our name being a prefix of the other name (to allow for, for example, 'Helvetica' -> 'Helvetica Bold')
        // Very bad match if we're not a prefix (which makes the score change)
        // Lots of combinations to check when there are a lot of names in both specs, but very often the 'other' is expected to be an exact match with just one name
        let name_diff = if !self.family_names.is_empty() && !other.family_names.is_empty() {
            // Check names against each other
            let mut name_diff = f64::MAX;
            let weight_suffix = self.weight_suffix();

            for our_name in self.family_names.iter() {
                // Might be called 'Helvetica Bold' or something, so append the possible weight suffixes
                let our_name_with_weight_suffix = weight_suffix.iter().map(|suffix| our_name.to_owned() + *suffix).collect::<Vec<_>>();

                for their_name in other.family_names.iter() {
                    let this_name_diff = if their_name.starts_with(our_name) {
                        if our_name == their_name {
                            // Exact match
                            0.0
                        } else if our_name_with_weight_suffix.iter().any(|our_name| our_name == their_name) {
                            // Right name but uses the weight suffix
                            0.0
                        } else {
                            // Difference in lengths (their_name must be longer because we know we're a prefix)
                            let diff = their_name.len() - our_name.len();

                            // Might be a suffix like '-Bold' that we don't support so only penalize differences a little
                            (diff as f64) * 10.0
                        }
                    } else {
                        // Not a good name: doesn't match at all
                        10000.0
                    };

                    name_diff = name_diff.min(this_name_diff);
                }
            }

            name_diff
        } else if !self.family_names.is_empty() && other.family_names.is_empty() {
            // Name unclear
            1000.0
        } else {
            // Our name can be anything
            0.0
        };

        // Final score is the sum of the scores
        weight_diff + style_diff + family_diff + name_diff
    }

    ///
    /// Returns a FontSpec for the system UI font on the current platform
    ///
    pub fn system_ui_font() -> Self {
        #[cfg(target_os = "windows")]
        {
            // Segoe UI is the standard Windows system UI font since Vista
            FontSpec::default()
                .with_family_name("Segoe UI")
                .with_alternative_family_name("Tahoma")
                .with_family(FontFamily::SansSerif)
        }

        #[cfg(any(target_os = "macos", target_os = "ios"))]
        {
            // San Francisco is the system UI font on macOS/iOS, accessible via .AppleSystemUIFont (except .AppleSystemUIFont causes a panic in font-kit)
            // (Apple protects this font so it's kind of hard to load into our font system, so we tend to fall back to Helvetica here. Some names for it
            // are listed here in case future versions of Mac OS are less picky)
            FontSpec::default()
                //.with_family_name(".AppleSystemUIFont")
                .with_alternative_family_name("SF Pro")     // Official name, not found
                .with_alternative_family_name("Helvetica Neue")
                .with_alternative_family_name("Helvetica")
                .with_family(FontFamily::SansSerif)
        }

        #[cfg(target_os = "android")]
        {
            // Roboto is the standard Android system UI font since Android 4.0
            FontSpec::default()
                .with_family_name("Roboto")
                .with_family(FontFamily::SansSerif)
        }

        #[cfg(target_os = "linux")]
        {
            // Linux system fonts vary by distro; try common ones in order
            FontSpec::default()
                .with_family_name("Ubuntu")
                .with_alternative_family_name("Cantarell")
                .with_alternative_family_name("DejaVu Sans")
                .with_alternative_family_name("Liberation Sans")
                .with_alternative_family_name("Noto Sans")
                .with_family(FontFamily::SansSerif)
        }

        #[cfg(not(any(
            target_os = "windows",
            target_os = "macos",
            target_os = "ios",
            target_os = "android",
            target_os = "linux",
        )))]
        {
            FontSpec::default().with_family(FontFamily::SansSerif)
        }
    }
}

impl Into<FontSpec> for &FontSpec {
    #[inline]
    fn into(self) -> FontSpec {
        self.clone()
    }
}

impl From<&str> for FontSpec {
    #[inline]
    fn from(name: &str) -> FontSpec {
        FontSpec::default().with_family_name(name)
    }
}

impl From<&String> for FontSpec {
    #[inline]
    fn from(name: &String) -> FontSpec {
        FontSpec::default().with_family_name(name)
    }
}

impl From<String> for FontSpec {
    #[inline]
    fn from(name: String) -> FontSpec {
        FontSpec::default().with_family_name(name)
    }
}

impl From<FontFamily> for FontSpec {
    #[inline]
    fn from(family: FontFamily) -> FontSpec {
        FontSpec::default().with_family(family)
    }
}

#[cfg(feature="outline-fonts")]
mod fontkit_font_spec {
    use super::*;
    use font_kit::properties::*;
    use font_kit::family_name::*;

    impl Into<Style> for FontStyle {
        fn into(self) -> Style {
            match self {
                FontStyle::Normal   => Style::Normal,
                FontStyle::Italic   => Style::Italic,
                FontStyle::Oblique  => Style::Oblique,
            }
        }
    }

    impl Into<Style> for &FontSpec {
        fn into(self) -> Style {
            match self.style {
                Some(style) => style.into(),
                None        => Style::default(),
            }
        }
    }

    impl Into<Weight> for &FontSpec {
        fn into(self) -> Weight {
            match self.weight {
                Some(weight) => Weight(weight as _),
                None         => Weight::default(),
            }
        }
    }

    impl Into<Stretch> for &FontSpec {
        fn into(self) -> Stretch {
            Stretch::default()
        }
    }

    ///
    /// Coverts a FontSpec into a list of possible font-kit family names
    ///
    impl Into<Vec<FamilyName>> for &FontSpec {
        fn into(self) -> Vec<FamilyName> {
            let font_family = match self.family {
                Some(FontFamily::Serif)     => vec![FamilyName::Serif],
                Some(FontFamily::SansSerif) => vec![FamilyName::SansSerif],
                Some(FontFamily::Monospace) => vec![FamilyName::Monospace],
                Some(FontFamily::Cursive)   => vec![FamilyName::Cursive],
                Some(FontFamily::Fantasy)   => vec![FamilyName::Fantasy],
                Some(FontFamily::SystemUI)  => (&FontSpec::system_ui_font()).into(),
                None                        => vec![],
            };

            self.family_names.iter()
                .map(|name| FamilyName::Title(name.clone()))
                .chain(font_family)
                .collect()
        }
    }

    ///
    /// Converts a FontSpec into font-kit properties
    ///
    impl Into<Properties> for &FontSpec {
        fn into(self) -> Properties {
            let mut properties = Properties::new();

            match self.style {
                Some(style) => { properties.style(style.into()); },
                None        => { },
            }

            match self.weight {
                Some(weight)    => { properties.weight(Weight(weight as _)); },
                None            => { }
            }

            properties
        }
    }

    impl Into<(Vec<FamilyName>, Properties)> for &FontSpec {
        #[inline]
        fn into(self) -> (Vec<FamilyName>, Properties) {
            (self.into(), self.into())
        }
    }

    impl Into<Properties> for FontSpec {
        #[inline]
        fn into(self) -> Properties {
            (&self).into()
        }
    }

    impl Into<(Vec<FamilyName>, Properties)> for FontSpec {
        #[inline]
        fn into(self) -> (Vec<FamilyName>, Properties) {
            ((&self).into(), (&self).into())
        }
    }

    impl From<(&[FamilyName], &Properties)> for FontSpec {
        fn from((family_names, properties): (&[FamilyName], &Properties)) -> FontSpec {
            let spec = family_names.iter()
                .fold(FontSpec::default(), |spec, family_name| {
                    match (family_name, &spec.family) {
                        (FamilyName::Title(name), _)    => spec.with_alternative_family_name(name),
                        (FamilyName::Serif, None)       => spec.with_family(FontFamily::Serif),
                        (FamilyName::SansSerif, None)   => spec.with_family(FontFamily::SansSerif),
                        (FamilyName::Monospace, None)   => spec.with_family(FontFamily::Monospace),
                        (FamilyName::Cursive, None)     => spec.with_family(FontFamily::Cursive),
                        (FamilyName::Fantasy, None)     => spec.with_family(FontFamily::Fantasy),
                        (_, Some(_))                    => spec,
                    }
                });

            let Weight(weight)  = properties.weight;
            let spec            = spec.with_weight(weight as _);

            let spec = spec.with_style(match properties.style {
                Style::Normal   => FontStyle::Normal,
                Style::Italic   => FontStyle::Italic,
                Style::Oblique  => FontStyle::Oblique,
            });

            spec
        }
    }
}
