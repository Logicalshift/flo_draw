//!
//! # EdgePlan
//!
//! An 'edge-plan' is an intermediate representation of a 2D scene that can be used to generate a 'scan-plan'. It
//! represents a scene as a series of edges that start or stop a program that generates pixels, optionally mixing
//! the new program with the old one. To rasterize an 'edge-plan', a simple ray-casting algorithm is applied.
//!
//! Anti-aliasing can be achieved by tracing an edge over sub-pixels and partially mixing in the new program where
//! it partially covers a pixel. A less-accurate form of anti-aliasing can also be used where we assume that the
//! edges are linked by 1-pixel high linear sections.
//!

mod edge_descriptor;
mod edge_id;
mod shape_descriptor;
mod shape_id;
mod edge_plan;
mod edge_intercept;

pub use edge_descriptor::*;
pub use edge_id::*;
pub use shape_descriptor::*;
pub use shape_id::*;
pub use edge_plan::*;
pub use edge_intercept::*;
