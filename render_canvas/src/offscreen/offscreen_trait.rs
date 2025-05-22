use flo_canvas::*;

///
/// Offscreen drawing contexts can be used to create offscreen drawing targets (which can be used to render `Draw` commands and generate a `Vec<u8>` containing the RGBA
/// data that results from rendering them)
///
pub trait OffscreenDrawingContext {
    /// Drawing targets are where canvas `Draw` instructions can be sent
    type DrawingTarget: OffscreenDrawingTarget;

    ///
    /// Creates a new drawing target for this context
    ///
    fn create_drawing_target(&mut self, width: usize, height: usize) -> Self::DrawingTarget;
}

///
/// An offscreen drawing target is used to send drawing commands and generate a buffer 
///
pub trait OffscreenDrawingTarget : GraphicsContext {
    ///
    /// Sends render actions to this offscreen render target
    ///
    fn draw_actions(&mut self, actions: impl IntoIterator<Item=Draw>) -> impl std::future::Future<Output = ()>;

    ///
    /// Consumes this render target and returns the realized pixels as a byte array
    ///
    fn realize(self) -> Vec<u8>;
}
