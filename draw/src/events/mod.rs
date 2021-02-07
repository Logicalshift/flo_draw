//!
//! # Events
//!
//! `flo_draw` is currently based on glutin, but uses its own event structure: this is to make it so that
//! it's possible for future versions to replace glutin easily if that ever proves to be necessary, and
//! to support easy porting of code using `flo_draw` to other windowing systems if that is ever necessary.
//!

mod pointer_event;
mod keypress;

pub use self::pointer_event::*;
pub use self::keypress::*;
