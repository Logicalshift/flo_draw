use super::error::*;
use super::opengl::*;
use super::offscreen_trait::*;

use flo_render_gl_offscreen::wgl;
use flo_render_gl_offscreen::winapi;
use flo_render_gl_offscreen::winapi::um::winuser::{CreateWindowExW, RegisterClassExW, GetDC, DefWindowProcW, WNDCLASSEXW, CS_OWNDC, WS_EX_APPWINDOW, WS_OVERLAPPEDWINDOW};
use flo_render_gl_offscreen::winapi::um::wingdi::{ChoosePixelFormat, SetPixelFormat, PIXELFORMATDESCRIPTOR, PFD_DRAW_TO_WINDOW, PFD_SUPPORT_OPENGL, PFD_DOUBLEBUFFER, PFD_TYPE_RGBA, PFD_MAIN_PLANE};
use flo_render_gl_offscreen::winapi::um::libloaderapi::{GetModuleHandleW};
use flo_render_gl_offscreen::winapi::um::errhandlingapi::{GetLastError};
use flo_render_gl_offscreen::winapi::shared::windef::{HWND};
use flo_render_gl_offscreen::winapi::shared::winerror::{ERROR_CLASS_ALREADY_EXISTS};

use std::mem;
use std::ptr;
use std::ffi;
use std::ffi::{CString, OsStr};
use std::os::windows::ffi::{OsStrExt};

struct WglOffscreenRenderContext {
    window: HWND
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
    unsafe {
        // Set up the window class
        let class_name              = OsStr::new("FloDraw Class").encode_wide().chain(Some(0).into_iter()).collect::<Vec<_>>();
        let window_name             = OsStr::new("flo_draw OpenGL window").encode_wide().chain(Some(0).into_iter()).collect::<Vec<_>>();

        let mut window_class        = mem::zeroed::<WNDCLASSEXW>();

        window_class.cbSize         = mem::size_of::<WNDCLASSEXW>() as u32;
        window_class.lpszClassName  = class_name.as_ptr();
        window_class.style          = CS_OWNDC;
        window_class.hInstance      = GetModuleHandleW(ptr::null());
        window_class.lpfnWndProc    = Some(DefWindowProcW);

        if RegisterClassExW(&window_class) == 0 {
            if GetLastError() != ERROR_CLASS_ALREADY_EXISTS {
                panic!("Unable to register window class for offscreen rendering: {}", GetLastError());
            }
        }

        // Create a window (we never show this or render to it, but we need a device context)
        let window                  = CreateWindowExW(WS_EX_APPWINDOW, class_name.as_ptr(), window_name.as_ptr(), WS_OVERLAPPEDWINDOW, 0, 0, 1024, 768, ptr::null_mut(), ptr::null_mut(), GetModuleHandleW(ptr::null()), ptr::null_mut());
        if window.is_null() {
            panic!("Unable to create window for offscreen rendering: {}", GetLastError());
        }

        // Fetch the device context for the window
        let dc                      = GetDC(window);
        if dc.is_null() {
            panic!("Unable to fetch device context for offscreen window: {}", GetLastError());
        }

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

        // Set the pixel format for the window
        let chosen_format = ChoosePixelFormat(dc, &pixel_format);
        SetPixelFormat(dc, chosen_format, &pixel_format);

        // Load the GL commands
        gl::load_with(|name| {
            unsafe {
                let name = CString::new(name.as_bytes()).unwrap();
                let name = name.as_ptr();

                wgl::wgl::GetProcAddress(name) as *const ffi::c_void
            }
        });

        // The result is a new WglOffscreenRenderContext
        Ok(WglOffscreenRenderContext {
            window: window
        })
    }
}

impl OffscreenRenderContext for WglOffscreenRenderContext {
    type RenderTarget = OpenGlOffscreenRenderer;

    ///
    /// Creates a new render target for this context
    ///
    fn create_render_target(&mut self, width: usize, height: usize) -> Self::RenderTarget {
        panic!("Cannot create WGL render target yet");

        unsafe {
            OpenGlOffscreenRenderer::new(width, height)
        }
    }
}
