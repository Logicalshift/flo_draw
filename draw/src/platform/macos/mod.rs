//!
//! To support a deeper feature set on OS X and fix some issues we use a custom view inside the
//! normal winit view.
//!
//! There are two main goals from this approach:
//!
//!  * Support pressure events from the view
//!  * Disable the transistions that occur when rendering the view
//!

mod flo_draw_view;
mod events;
mod platform_subprogram;

pub (crate) use flo_draw_view::*;
pub (crate) use platform_subprogram::*;

#[cfg(feature="render-wgpu")] pub (crate) use objc2_quartz_core::{CATransaction};
