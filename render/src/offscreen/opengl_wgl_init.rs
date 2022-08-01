use super::error::*;
use super::opengl::*;
use super::offscreen_trait::*;

use flo_render_gl_offscreen::wgl;
use flo_render_gl_offscreen::winapi::um::winuser::{CreateWindowExW, RegisterClassExW, GetDC, DefWindowProcW, DestroyWindow, WNDCLASSEXW, CS_OWNDC, WS_EX_APPWINDOW, WS_OVERLAPPEDWINDOW};
use flo_render_gl_offscreen::winapi::um::wingdi::{ChoosePixelFormat, SetPixelFormat, PIXELFORMATDESCRIPTOR, PFD_DRAW_TO_WINDOW, PFD_SUPPORT_OPENGL, PFD_DOUBLEBUFFER, PFD_TYPE_RGBA, PFD_MAIN_PLANE};
use flo_render_gl_offscreen::winapi::um::libloaderapi::{LoadLibraryW, FreeLibrary, GetModuleHandleW, GetProcAddress};
use flo_render_gl_offscreen::winapi::um::errhandlingapi::{GetLastError};
use flo_render_gl_offscreen::winapi::shared::windef::{HWND, HGLRC};
use flo_render_gl_offscreen::winapi::shared::minwindef::{HINSTANCE};
use flo_render_gl_offscreen::winapi::shared::winerror::{ERROR_CLASS_ALREADY_EXISTS};

use std::mem;
use std::ptr;
use std::ffi;
use std::ffi::{CString, OsStr};
use std::os::windows::ffi::{OsStrExt};

///
/// Represents a WGL off-screen rendering context
///
struct WglOffscreenRenderContext {
    /// The window that owns the device context
    window: HWND,

    /// The OpenGL context
    context: HGLRC,

    /// The OpenGL library module
    opengl_library: HINSTANCE
}

///
/// Performs on-startup initialisation steps for offscreen rendering
///
/// Only required if not using a toolkit renderer (eg, in an HTTP renderer or command-line tool). Will likely replace
/// the bindings for any GUI toolkit, so this is not appropriate for desktop-type apps.
///
/// This version is the WGL version for Windows
///
pub fn opengl_initialize_offscreen_rendering() -> Result<impl OffscreenRenderContext, RenderInitError> {
    // See also https://www.khronos.org/opengl/wiki/Creating_an_OpenGL_Context_(WGL)
    unsafe {
        // Set up the window class
        let class_name              = OsStr::new("FloRender Class").encode_wide().chain(Some(0).into_iter()).collect::<Vec<_>>();
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

        // Create a window (we never show this or render to it, but we need a device context to initialise OpenGL)
        let window                  = CreateWindowExW(WS_EX_APPWINDOW, class_name.as_ptr(), window_name.as_ptr(), WS_OVERLAPPEDWINDOW, 0, 0, 1024, 768, ptr::null_mut(), ptr::null_mut(), GetModuleHandleW(ptr::null()), ptr::null_mut());
        if window.is_null() {
            panic!("Unable to create window for offscreen rendering: {}", GetLastError());
        }

        // Fetch the device context for the window
        let dc              = GetDC(window);
        if dc.is_null() {
            panic!("Unable to fetch device context for offscreen window: {}", GetLastError());
        }

        // Set up the pixel format descriptor
        let pixel_format    = PIXELFORMATDESCRIPTOR {
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
        let chosen_format   = ChoosePixelFormat(dc, &pixel_format);
        SetPixelFormat(dc, chosen_format, &pixel_format);

        // Create a context for the window and make it current (this is to load the extra functions, the default context is an old version of OpenGL...)
        let initial_ctxt    = wgl::wgl::CreateContext(dc as *const _);
        if initial_ctxt.is_null() {
            panic!("Could not create initial OpenGL context: {}", GetLastError());
        }

        if wgl::wgl::MakeCurrent(dc as *const _, initial_ctxt) == 0 {
            panic!("Could not make initial context current: {}", GetLastError());
        }

        // Load the extra functions
        let extra_functions = wgl::wgl_extra::Wgl::load_with(|addr| {
            let addr = CString::new(addr.as_bytes()).unwrap();
            let addr = addr.as_ptr();
            wgl::wgl::GetProcAddress(addr) as *const _
        });

        // Done with the initial context
        wgl::wgl::MakeCurrent(dc as *const _, ptr::null());
        wgl::wgl::DeleteContext(initial_ctxt);

        // Create an OpenGL 3.3 context to use for rendering using the extra functions
        let mut opengl33_attribs = vec![];
        opengl33_attribs.push(wgl::wgl_extra::CONTEXT_MAJOR_VERSION_ARB as i32);
        opengl33_attribs.push(3);
        opengl33_attribs.push(wgl::wgl_extra::CONTEXT_MINOR_VERSION_ARB as i32);
        opengl33_attribs.push(3);
        opengl33_attribs.push(0);

        let ctxt = extra_functions.CreateContextAttribsARB(dc as *const _, ptr::null(), opengl33_attribs.as_ptr());
        if ctxt.is_null() {
            panic!("Could not create OpenGL 3.3 context: {}", GetLastError());
        }

        if wgl::wgl::MakeCurrent(dc as *const _, ctxt) == 0 {
            panic!("Could not make OpenGL 3.3 context current: {}", GetLastError());
        }

        // Load the OpenGL library
        let opengl_name     = OsStr::new("opengl32.dll").encode_wide().chain(Some(0).into_iter()).collect::<Vec<_>>();
        let opengl_library  = LoadLibraryW(opengl_name.as_ptr());

        // Load the GL commands
        gl::load_with(|name| {
            // Convert the name to a CString
            let name = CString::new(name.as_bytes()).unwrap();
            let name = name.as_ptr();

            // Try to load via the WGL function#
            let proc_address = wgl::wgl::GetProcAddress(name) as *const ffi::c_void;
            if !proc_address.is_null() {
                proc_address
            } else {
                // Try to load via the usual windows function from the OpenGL library
                GetProcAddress(opengl_library, name) as *const _
            }
        });

        // Unset the context
        wgl::wgl::MakeCurrent(dc as *const _, ptr::null());

        // The result is a new WglOffscreenRenderContext
        Ok(WglOffscreenRenderContext {
            window:         window,
            context:        ctxt as HGLRC,
            opengl_library: opengl_library
        })
    }
}

impl Drop for WglOffscreenRenderContext {
    fn drop(&mut self) {
        unsafe {
            // Delete the context
            wgl::wgl::DeleteContext(self.context as *const _);

            // Delete the window
            DestroyWindow(self.window);

            // Close the library
            FreeLibrary(self.opengl_library);
        }
    }
}

impl OffscreenRenderContext for WglOffscreenRenderContext {
    type RenderTarget = OpenGlOffscreenRenderer;

    ///
    /// Creates a new render target for this context
    ///
    fn create_render_target(&mut self, width: usize, height: usize) -> Self::RenderTarget {
        unsafe {
            // Get the window device context...
            let dc  = GetDC(self.window);
            if dc.is_null() {
                panic!("Unable to fetch device context for offscreen window: {}", GetLastError());
            }

            // Make the openGL context for the window current
            if wgl::wgl::MakeCurrent(dc as *const _, self.context as *const _) == 0 {
                panic!("Could not make OpenGL context current: {}", GetLastError());
            }

            // Renderer is reader
            OpenGlOffscreenRenderer::new(width, height)
        }
    }
}
