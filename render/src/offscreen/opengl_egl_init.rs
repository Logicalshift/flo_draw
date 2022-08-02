use super::error::*;
use super::opengl::*;
use super::offscreen_trait::*;

use gl;
use flo_render_gl_offscreen::egl;
use flo_render_gl_offscreen::egl::ffi;
use flo_render_gl_offscreen::gbm;
use libc::{open, close, O_RDWR};

use std::ptr;
use std::ffi::{CString, c_void};

///
/// An OpenGL offscreen rendering context initialised by EGL
///
struct EglOffscreenRenderContext {
    /// The file descriptor of the DRI file for the graphics card we're using to render
    card_fd: i32,

    /// The EGL display that we created
    display: egl::EGLDisplay,

    /// The rendering context
    context: egl::EGLContext,
}

///
/// Performs on-startup initialisation steps for offscreen rendering
///
/// Only required if not using a toolkit renderer (eg, in an HTTP renderer or command-line tool). Will likely replace
/// the bindings for any GUI toolkit, so this is not appropriate for desktop-type apps.
///
/// This version is the EGL version for Linux
///
pub fn opengl_initialize_offscreen_rendering() -> Result<impl OffscreenRenderContext, RenderInitError> {
    unsafe {
        // Open the card0 file descriptor
        let card0_file = CString::new("/dev/dri/card0").unwrap();
        let card0 = open(card0_file.as_ptr(), O_RDWR);
        if card0 == 0 { Err(RenderInitError::CannotOpenGraphicsDevice)? }

        // Create the GBM device for the card
        let gbm = gbm::gbm_create_device(card0);
        if gbm.is_null() { Err(RenderInitError::CannotCreateGraphicsDevice)? }

        // Initialise EGL
        if !egl::bind_api(egl::EGL_OPENGL_API) { Err(RenderInitError::ApiNotAvailable)? }

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
        let config = egl::choose_config(egl_display, &[
                egl::EGL_RED_SIZE,          8,
                egl::EGL_GREEN_SIZE,        8,
                egl::EGL_BLUE_SIZE,         8,
                egl::EGL_DEPTH_SIZE,        24,
                egl::EGL_CONFORMANT,        egl::EGL_OPENGL_BIT,
                egl::EGL_RENDERABLE_TYPE,   egl::EGL_OPENGL_BIT, 
                egl::EGL_NONE
            ], 1);
        let config = if let Some(config) = config { config } else { println!("egl::choose_config {:x}", egl::get_error()); Err(RenderInitError::CouldNotConfigureDisplay)? };

        // Create the context
        let context = egl::create_context(egl_display, config, egl::EGL_NO_CONTEXT, &[
                egl::EGL_CONTEXT_MAJOR_VERSION, 3, 
                egl::EGL_CONTEXT_MINOR_VERSION, 3, 
                egl::EGL_NONE
            ]);
        let context = if let Some(context) = context { context } else { println!("egl::create_context {:x}", egl::get_error()); Err(RenderInitError::CouldNotCreateContext)? };

        // End with this set as the current context
        let activated_context = egl::make_current(egl_display, egl::EGL_NO_SURFACE, egl::EGL_NO_SURFACE, context);

        if !activated_context { println!("egl::make_current {:x}", egl::get_error()); Err(RenderInitError::ContextDidNotStart)? }

        // Set up the GL funcitons and check for errors
        gl::load_with(|s| egl::get_proc_address(s) as *const c_void);
        let error = gl::GetError();
        if error != gl::NO_ERROR { println!("gl::GetError {:x}", error); Err(RenderInitError::ContextDidNotStart)? }
        assert!(error == gl::NO_ERROR);

        Ok(EglOffscreenRenderContext {
            card_fd: card0,
            display: egl_display,
            context: context
        })
    }
}

///
/// Performs on-startup initialisation steps for offscreen rendering
///
/// Only required if not using a toolkit renderer (eg, in an HTTP renderer or command-line tool). Will likely replace
/// the bindings for any GUI toolkit, so this is not appropriate for desktop-type apps.
///
/// This version is the Metal version for Mac OS X
///
#[cfg(not(feature="osx-metal"))]
pub fn initialize_offscreen_rendering() -> Result<impl OffscreenRenderContext, RenderInitError> {
    opengl_initialize_offscreen_rendering()
}

impl OffscreenRenderContext for EglOffscreenRenderContext {
    type RenderTarget = OpenGlOffscreenRenderer;

    ///
    /// Creates a new render target for this context
    ///
    fn create_render_target(&mut self, width: usize, height: usize) -> Self::RenderTarget {
        let activated_context = egl::make_current(self.display, egl::EGL_NO_SURFACE, egl::EGL_NO_SURFACE, self.context);
        if !activated_context { panic!("egl::make_current {:x}", egl::get_error()); }

        OpenGlOffscreenRenderer::new(width, height)
    }
}

impl Drop for EglOffscreenRenderContext {
    fn drop(&mut self) {
        unsafe {
            egl::destroy_context(self.display, self.context);
            close(self.card_fd);
        }
    }
}
