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
//! `DrawWindowRequest` provides a set of requests for interacting directly with the window: this is mainly a way
//! to obtain the events and rendering event channels for a particular window.
//!

mod render_window_entity;
mod drawing_window_entity;
mod scene;

mod glutin_render_window_entity;
mod glutin_scene;

pub use self::render_window_entity::*;
pub use self::drawing_window_entity::*;
pub use self::scene::*;
