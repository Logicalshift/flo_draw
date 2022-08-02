#[cfg(feature="render-wgpu")] #[macro_use] extern crate lazy_static;

mod action;
mod buffer;
#[cfg(feature="gl")] mod gl_renderer;
#[cfg(feature="osx-metal")] mod metal_renderer;
#[cfg(feature="render-wgpu")] mod wgpu_renderer;
mod offscreen;

pub use self::action::*;
pub use self::buffer::*;
pub use self::offscreen::*;
#[cfg(feature="gl")] pub use self::gl_renderer::{GlRenderer};
#[cfg(feature="osx-metal")] pub use self::metal_renderer::{MetalRenderer};
#[cfg(feature="render-wgpu")] pub use self::wgpu_renderer::{WgpuRenderer};

#[cfg(feature="render-wgpu")]
pub use wgpu;
