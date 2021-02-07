//!
//! `flo_draw` provides a simple API for rendering 2D graphics
//!

#[macro_use] extern crate lazy_static;

pub use flo_canvas as canvas;
pub use flo_render_canvas as render_canvas;
pub use flo_render::{initialize_offscreen_rendering};
pub use flo_render_canvas::{render_canvas_offscreen};

mod events;
mod draw_event;
mod canvas_window;
mod render_window;
mod glutin_thread;
mod glutin_window;
mod glutin_runtime;
mod window_properties;
mod glutin_thread_event;

pub use self::events::*;
pub use self::draw_event::*;
pub use self::canvas_window::*;
pub use self::render_window::*;
pub use self::glutin_thread::{with_2d_graphics};
pub use self::window_properties::*;
