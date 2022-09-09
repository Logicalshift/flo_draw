//!
//! # Events
//!
//! `flo_draw` is currently based on glutin, but uses its own event structure: this is to make it so that
//! it's possible for future versions to replace glutin easily if that ever proves to be necessary, and
//! to support easy porting of code using `flo_draw` to other windowing systems. This also isolates software
//! implemented using `flo_draw` from changes to glutin.
//!

mod key;
mod draw_event;
mod pointer_event;

mod render_request;
mod draw_event_request;

mod draw_window_request;

pub use self::key::*;
pub use self::draw_event::*;
pub use self::pointer_event::*;

pub use self::render_request::*;
pub use self::draw_event_request::*;

pub use self::draw_window_request::*;
