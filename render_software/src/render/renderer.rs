///
/// A renderer converts from a set of source instructions to a set of destination values
///
pub trait Renderer {
    type Source;
    type Dest;

    ///
    /// Renders a set of instructions to a destination
    ///
    fn render(&self, source: &Self::Source, dest: &mut Self::Dest);
}
