//!
//! # flo_canvas
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
//! # Features
//!
//! Some features of `flo_canvas` are optional due to the extra dependencies they can bring in. They
//! can be enabled if the extra functionality is required or left out to get a more compact library.
//!
//! * `outline-fonts` - provides a function that will convert a stream of Draw instructions into
//!   another stream of Draw instructions, except all the font commands will be removed and replaced
//!   with an outline rendering of the font (useful for rendering back-ends that don't have native
//!   font support or for generating vector files that don't require particular fonts to be installed)
//!
#![warn(bare_trait_objects)]

#[macro_use]
extern crate serde_derive;

extern crate futures;
extern crate desync;
extern crate hsluv;

mod draw;
mod font;
mod color;
mod canvas;
mod context;
mod texture;
mod encoding;
mod decoding;
mod gradient;
mod font_face;
mod primitives;
mod transform2d;
mod draw_stream;
mod draw_resource;
mod drawing_target;
mod conversion_streams;

#[cfg(feature = "outline-fonts")] mod font_line_layout;

pub use self::draw::*;
pub use self::font::*;
pub use self::color::*;
pub use self::canvas::*;
pub use self::context::*;
pub use self::texture::*;
pub use self::encoding::*;
pub use self::decoding::*;
pub use self::gradient::*;
pub use self::font_face::*;
pub use self::primitives::*;
pub use self::transform2d::*;
pub use self::draw_stream::*;
pub use self::drawing_target::*;
pub use self::conversion_streams::*;

#[cfg(feature = "outline-fonts")] pub use self::font_line_layout::*;

pub use flo_curves as curves;
pub use flo_curves::geo::{Coordinate2D, Coord2};
