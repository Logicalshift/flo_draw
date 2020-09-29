mod error;
mod offscreen_trait;

#[cfg(feature="opengl")]                                                    mod opengl;
#[cfg(all(feature="opengl", target_os = "linux"))]                          mod opengl_egl_init;
#[cfg(all(feature="opengl", target_os = "macos", not(feature="metal")))]    mod opengl_cgl_init;
#[cfg(all(feature="osx-metal", target_os = "macos"))]                       mod metal_init;

pub use self::error::*;
pub use self::offscreen_trait::*;

#[cfg(all(feature="opengl", target_os = "linux"))]                          pub use self::opengl_egl_init::*;
#[cfg(all(feature="opengl", target_os = "macos", not(feature="metal")))]    pub use self::opengl_cgl_init::*;
#[cfg(all(feature="osx-metal", target_os = "macos"))]                       pub use self::metal_init::*;

#[cfg(test)] mod test;
