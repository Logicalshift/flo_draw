use super::error::*;

use gl;
use flo_render_gl_offscreen::egl;
use flo_render_gl_offscreen::gbm;
use libc::{open, O_RDWR};

use std::ptr;
use std::ffi::{CString};

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
