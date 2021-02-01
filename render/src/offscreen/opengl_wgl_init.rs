use super::error::*;
use super::opengl::*;
use super::offscreen_trait::*;

use flo_render_gl_offscreen::wgl;
use flo_render_gl_offscreen::winapi;

struct WglOffscreenRenderContext {

}

///
/// Performs on-startup initialisation steps for offscreen rendering
///
/// Only required if not using a toolkit renderer (eg, in an HTTP renderer or command-line tool). Will likely replace
/// the bindings for any GUI toolkit, so this is not appropriate for desktop-type apps.
///
/// This version is the WGL version for Windows
///
pub fn initialize_offscreen_rendering() -> Result<impl OffscreenRenderContext, RenderInitError> {
	// The result is a new WglOffscreenRenderContext
	Ok(WglOffscreenRenderContext {

	})
}

impl OffscreenRenderContext for WglOffscreenRenderContext {
    type RenderTarget = OpenGlOffscreenRenderer;

    ///
    /// Creates a new render target for this context
    ///
    fn create_render_target(&mut self, width: usize, height: usize) -> Self::RenderTarget {
        unsafe {
            OpenGlOffscreenRenderer::new(width, height)
        }
    }
}
