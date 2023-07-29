mod pixel_program;
mod pixel_program_cache;

/// Kinds of edges that can be used in an edge plan
pub mod edges;

/// An edge plan divides a 2D spaces into regions using arbitrary edge definitions, and can be rendered down into a scan plan
pub mod edgeplan;

/// A scan plan describes the actions required to draw a single scanline (modelling a 1 dimensional space)
pub mod scanplan;

/// A pixel models a single colour sample (thematically it could be considered 0 dimensional, though really a pixel is better modelled as aggregation of the light passing through a particular region)
pub mod pixel;

pub use pixel_program::*;
pub use pixel_program_cache::*;
