///
/// A renderer converts from a set of source instructions to a set of destination values
///
pub trait Renderer : Send + Sync {
    /// The region is used to specify what region is being rendered
    type Region: ?Sized;

    /// The source is the source instructions for the rendering
    type Source: ?Sized;

    /// The dest is the target buffer type for the rendering
    type Dest: ?Sized;

    ///
    /// Renders a set of instructions to a destination
    ///
    fn render(&self, region: &Self::Region, source: &Self::Source, dest: &mut Self::Dest);
}
