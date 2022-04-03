///
/// The state a dynamic texture was in the last time it was rendered
///
#[derive(PartialEq)]
pub struct DynamicTextureState {
    /// The viewport size for the texture
    pub (super) viewport: (f32, f32),

    /// The number of times the sprite was modified
    pub (super) sprite_modification_count: usize
}