///
/// The types of render target that can be created by the render layer
///
#[derive(Clone, Copy, PartialEq, Debug, Hash)]
pub enum RenderTargetType {
    /// Standard off-screen render target (with a texture)
    Standard,

    /// Multisampled render target
    Multisampled,

    /// Multisampled texture render target
    MultisampledTexture,

    /// Monochrome off-screen render target (only writes the red channel)
    Monochrome,

    /// Multisampled monochrome off-screen render target (only writes the red channel)
    MonochromeMultisampledTexture
}
