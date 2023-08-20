///
/// A renderer converts from a set of source instructions to a set of destination values
///
pub trait Renderer {
    type Source: ?Sized;
    type Dest: ?Sized;

    ///
    /// Renders a set of instructions to a destination
    ///
    fn render(&self, source: &Self::Source, dest: &mut Self::Dest);
}

///
/// A line renderer is a renderer that renders single lines from a source to a destination
///
pub trait LineRenderer {
    type Source: ?Sized;
    type Dest: ?Sized;

    ///
    /// Renders a set of instructions to a destination
    ///
    fn render(&self, y_pos: f64, source: &Self::Source, dest: &mut Self::Dest);
}