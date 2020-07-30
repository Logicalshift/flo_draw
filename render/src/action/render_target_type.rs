///
/// The types of render target that can be created by the render layer
///
#[derive(Clone, Copy, PartialEq, Debug, Hash)]
pub enum RenderTargetType {
    /// Standard off-screen render target
    Standard,

    /// Multisampled render target
    Multisampled,

    /// Multisampled texture render target
    MultisampledTexture
}
