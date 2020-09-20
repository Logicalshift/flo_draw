use super::error::*;

use gl;
use flo_render_gl_offscreen::egl;
use flo_render_gl_offscreen::egl::ffi;
use flo_render_gl_offscreen::gbm;
use libc::{open, O_RDWR};

use std::ptr;
use std::ffi::{CString, c_void};

///
/// Performs on-startup initialisation steps for offscreen rendering
///
/// Only required if not using a toolkit renderer (eg, in an HTTP renderer or command-line tool). Will likely replace
/// the bindings for any GUI toolkit, so this is not appropriate for desktop-type apps.
///
/// This version is the EGL version for Linux
///
pub fn initialize_offscreen_rendering() -> Result<(), RenderInitError> {
    unsafe {
        // Open the card0 file descriptor
        let card0 = open(CString::new("/dev/dri/card0").unwrap().as_ptr(), O_RDWR);
        if card0 == 0 { Err(RenderInitError::CannotOpenGraphicsDevice)? }

        // Create the GBM device for the card
        let gbm = gbm::gbm_create_device(card0);
        if gbm.is_null() { Err(RenderInitError::CannotCreateGraphicsDevice)? }

        // Initialise EGL
        // let egl_display = ffi::eglGetPlatformDisplay(egl::EGL_PLATFORM_SURFACELESS_MESA, ptr::null_mut(), ptr::null());

        let egl_display = ffi::eglGetPlatformDisplay(egl::EGL_PLATFORM_GBM_MESA, gbm as *mut c_void, ptr::null());
        let egl_display = if egl_display.is_null() { None } else { Some(egl_display) };
        let egl_display = if let Some(egl_display) = egl_display { egl_display } else { println!("eglGetPlatformDisplay {:x}", egl::get_error()); Err(RenderInitError::DisplayNotAvailable)? };

        let mut major = 0;
        let mut minor = 0;
        let init_result = egl::initialize(egl_display as *mut c_void, &mut major, &mut minor);
        if !init_result { println!("egl::initialize {:x}", egl::get_error()); Err(RenderInitError::CannotStartGraphicsDriver)? }

        // Check for the create context and surfaceless extensions
        let extensions = egl::query_string(egl_display, egl::EGL_EXTENSIONS);
        let extensions = if let Some(extensions) = extensions { extensions } else { Err(RenderInitError::MissingRequiredExtension)? };
        let extensions = extensions.to_string_lossy();

        if !extensions.contains("EGL_KHR_create_context ")      { Err(RenderInitError::MissingRequiredExtension)? }
        if !extensions.contains("EGL_KHR_surfaceless_context ") { Err(RenderInitError::MissingRequiredExtension)? }

        // Pick the configuration
        let config = egl::choose_config(egl_display, &[egl::EGL_RENDERABLE_TYPE, egl::EGL_OPENGL_BIT, egl::EGL_NONE], 1);
        let config = if let Some(config) = config { config } else { Err(RenderInitError::CouldNotConfigureDisplay)? };

        // Create the context
        let context = egl::create_context(egl_display, config, egl::EGL_NO_CONTEXT, &[egl::EGL_CONTEXT_CLIENT_VERSION, 3, egl::EGL_NONE]);
        let context = if let Some(context) = context { context } else { println!("egl::create_context {:x}", egl::get_error()); Err(RenderInitError::CouldNotCreateContext)? };

        // End with this set as the current context
        let activated_context = egl::make_current(egl_display, egl::EGL_NO_SURFACE, egl::EGL_NO_SURFACE, context);
        gl::load_with(|s| egl::get_proc_address(s) as *const c_void);

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn init_opengl_subsystem() {
        assert!(initialize_offscreen_rendering().is_ok());
    }
}
