//!
//! # CurvePlan
//!
//! A 'curve-plan' is an intermediate representation of a 2D scene that can be used to generate a 'scan-plan'. It
//! represents a scene as a series of edges that start or stop a program that generates pixels, optionally mixing
//! the new program with the old one. To rasterize a 'curve-plan', a simple ray-casting algorithm is applied.
//!
//! Anti-aliasing can be achieved by tracing an edge over sub-pixels and partially mixing in the new program where
//! it partially covers a pixel. A less-accurate form of anti-aliasing can also be used where we assume that the
//! curves are linked by 1-pixel high linear sections.
//!
