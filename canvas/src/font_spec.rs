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
