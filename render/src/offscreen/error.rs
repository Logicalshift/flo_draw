#[allow(dead_code)]

///
/// Errors that can happen when trying to initialise the renderer
///
#[derive(Clone, Debug, PartialEq)]
pub enum RenderInitError {
    /// Indicates that the graphics device could not be opened
    CannotOpenGraphicsDevice,

    /// Indicates that the graphics device could not be attached to
    CannotCreateGraphicsDevice
}
