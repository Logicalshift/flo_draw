pub use ::egl::*;

#[link(name = "EGL")]
extern { }

//
// Constants missing from the EGL crate
//

pub const EGL_PLATFORM_GBM_KHR:             EGLenum = 0x31D7;
pub const EGL_PLATFORM_WAYLAND_KHR:         EGLenum = 0x31D8;
pub const EGL_PLATFORM_X11_KHR:             EGLenum = 0x31D5;
pub const EGL_PLATFORM_X11_SCREEN_KHR:      EGLenum = 0x31D6;
pub const EGL_PLATFORM_DEVICE_EXT:          EGLenum = 0x313F;
pub const EGL_PLATFORM_WAYLAND_EXT:         EGLenum = 0x31D8;
pub const EGL_PLATFORM_X11_EXT:             EGLenum = 0x31D5;
pub const EGL_PLATFORM_X11_SCREEN_EXT:      EGLenum = 0x31D6;
pub const EGL_PLATFORM_GBM_MESA:            EGLenum = 0x31D7;
pub const EGL_PLATFORM_SURFACELESS_MESA:    EGLenum = 0x31DD;

//
// FFI functions missing from the EGL crate
//

pub mod ffi {
    use ::egl::*;
    use std::ffi::{c_void};

    extern {
        pub fn eglGetPlatformDisplay(platform: EGLenum, native_display: *mut c_void, attributes: *const EGLint) -> EGLDisplay;
    }
}
