///
/// Layout for the TextureSettings uniform
///
#[derive(Clone, Copy, PartialEq, Debug)]
#[repr(C, packed)]
pub struct TextureSettings {
    pub transform: [[f32; 4]; 4],
    pub alpha: f32,
}
