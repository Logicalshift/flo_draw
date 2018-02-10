//!
//! An abstract representation of a vector canvas object 
//!

#[macro_use]
extern crate serde_derive;

extern crate futures;
extern crate curves;
extern crate desync;
extern crate hsluv;

mod gc;
mod draw;
mod color;
mod canvas;
mod encoding;
mod transform2d;

pub use self::gc::*;
pub use self::draw::*;
pub use self::color::*;
pub use self::canvas::*;
pub use self::encoding::*;
pub use self::transform2d::*;
