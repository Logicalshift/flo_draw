//!
//! The draw_scene module provides an interface to flo_draw using the flo_scene library.
//! 
//! `flo_scene` is a message and property passing framework. It's good for developing more complex applications
//! with flo_draw, such as those with a user interface.
//!
//! There are three main types of request: a `RenderRequest` which is a request for low-level graphics operations,
//! a `DrawRequest` which is a request for a high-level 2D graphics operation, and a `DrawEventRequest` which is
//! a request in the other direction to process a user interaction.
//!

mod render_request;
mod drawing_request;
mod draw_event_request;

pub use self::render_request::*;
pub use self::drawing_request::*;
pub use self::draw_event_request::*;
