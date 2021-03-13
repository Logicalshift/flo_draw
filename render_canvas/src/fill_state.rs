use flo_render as render;

///
/// The ways the next path can be filled
///
#[derive(Clone, PartialEq)]
pub enum FillState {
    ///
    /// Fill with a solid colour
    ///
    Color(render::Rgba8),
}

impl FillState {
    ///
    /// Returns a variant of this fill state with all channels set as the alpha channel
    ///
    pub fn all_channel_alpha(&self) -> Self {
        match self {
            FillState::Color(color) => FillState::Color(render::Rgba8([color.0[3], color.0[3], color.0[3], color.0[3]]))
        }
    }

    ///
    /// Returns the flat colour to use for this fill state
    ///
    pub fn flat_color(&self) -> render::Rgba8 {
        match self {
            FillState::Color(color) => *color
        }
    }
}