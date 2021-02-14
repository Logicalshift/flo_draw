//!
//! `flo_canvas` provides an abstraction of a 2D vector canvas, and supporting methods to stream
//! updates to implementations.
//!
//! The main features that this library supports are the set of primitives in the `Draw` enum, the
//! `Canvas` type for streaming drawing instructions elsewhere, and the encoding and decoding
//! functions that can be used to send canvas instructions over a byte stream. Encoding uses MIME64
//! characters, so it's easy to embed encoded canvases in other protocols.
//!
//! By itself, `flo_canvas` is an excellent way to describe how a 2D scene should be rendered without
//! needing to depend on a system-specific library.
//!
//! FlowBetween comes with several implementations of the canvas for generating the final rendered
//! results. Most notably, `flo_render_canvas` will convert between a stream of `Draw` instructions
//! and a stream of instructions suitable for rendering with most graphics APIs. The accompanying
//! `flo_render` can render these instructions to OpenGL or Metal and `flo_render_gl_offscreen` is
//! available to generate bitmap images on a variety of systems.
//!
//! `canvas.js` provides a Javascript implementation that can render the instructions to a HTML 
//! canvas, and there are also Quartz and Cairo implementations of the canvas provided in FlowBetween's
//! user interface layers.
//!
#![warn(bare_trait_objects)]

#[macro_use]
extern crate serde_derive;

extern crate futures;
extern crate flo_curves;
extern crate desync;
extern crate hsluv;

mod gc;
mod draw;
mod color;
mod canvas;
mod encoding;
mod decoding;
mod transform2d;

pub use self::gc::*;
pub use self::draw::*;
pub use self::color::*;
pub use self::canvas::*;
pub use self::encoding::*;
pub use self::decoding::*;
pub use self::transform2d::*;
