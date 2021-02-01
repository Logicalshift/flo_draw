use super::error::*;
use super::opengl::*;
use super::offscreen_trait::*;

use flo_render_gl_offscreen::wgl;
use flo_render_gl_offscreen::winapi;
use flo_render_gl_offscreen::winapi::um::winuser::{CreateWindowExW};
use flo_render_gl_offscreen::winapi::um::wingdi::{PIXELFORMATDESCRIPTOR, PFD_DRAW_TO_WINDOW, PFD_SUPPORT_OPENGL, PFD_DOUBLEBUFFER, PFD_TYPE_RGBA, PFD_MAIN_PLANE};

use std::mem;

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
    // See also https://www.khronos.org/opengl/wiki/Creating_an_OpenGL_Context_(WGL)

    // Create a window (we never show this or render to it, but we need a device context)
    // let window = 

    // Set up the pixel format descriptor
    let pixel_format = PIXELFORMATDESCRIPTOR {
        nSize:              mem::size_of::<PIXELFORMATDESCRIPTOR>() as u16,
        nVersion:           1,
        dwFlags:            PFD_DRAW_TO_WINDOW | PFD_SUPPORT_OPENGL | PFD_DOUBLEBUFFER,
        iPixelType:         PFD_TYPE_RGBA,
        cColorBits:         32,
        cRedBits:           0,
        cRedShift:          0,
        cGreenBits:         0,
        cGreenShift:        0,
        cBlueBits:          0,
        cBlueShift:         0,
        cAlphaBits:         0,
        cAlphaShift:        0,
        cAccumBits:         0,
        cAccumRedBits:      0,
        cAccumGreenBits:    0,
        cAccumBlueBits:     0,
        cAccumAlphaBits:    0,
        cDepthBits:         24,
        cStencilBits:       8,
        cAuxBuffers:        0,
        iLayerType:         PFD_MAIN_PLANE,
        bReserved:          0,
        dwLayerMask:        0,
        dwVisibleMask:      0,
        dwDamageMask:       0
    };

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
