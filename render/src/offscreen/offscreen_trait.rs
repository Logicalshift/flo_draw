use crate::action::*;

///
/// Trait implemented by FlowBetween offscreen render targets
///
pub trait OffscreenRenderTarget {
    ///
    /// Sends render actions to this offscreen render target
    ///
    fn render<ActionIter: IntoIterator<Item=RenderAction>>(&mut self, actions: ActionIter);

    ///
    /// Consumes this render target and returns the realized pixels as a byte array
    ///
    fn realize(self) -> Vec<u8>;
}
