use super::offscreen_trait::*;

use crate::action::*;
use crate::gl_renderer::*;

use gl;

use std::ffi::{c_void};

///
/// An offscreen renderer that uses OpenGL (whichever variety is initialised)
///
pub struct OpenGlOffscreenRenderer {
    /// The width of the main render target
    width: usize,

    /// The height of the main render target
    height: usize,

    /// The render target that the result is rendered to
    main_render_target: RenderTarget,

    /// The renderer, as it is set up for the current render target
    renderer: GlRenderer
}

impl OpenGlOffscreenRenderer {
    ///
    /// Creates a new OpenGL offscreen renderer with a texture of the specified size
    ///
    pub fn new(width: usize, height: usize) -> OpenGlOffscreenRenderer {
        // Create the main render target
        let main_render_target  = RenderTarget::new(width as u16, height as u16, RenderTargetType::Standard);

        // Set up the renderer to render to this target
        let renderer            = GlRenderer::new();

        // Generate the offscreen renderer
        OpenGlOffscreenRenderer {
            width:              width,
            height:             height,
            main_render_target: main_render_target,
            renderer:           renderer
        }
    }
}

impl OffscreenRenderTarget for OpenGlOffscreenRenderer {
    ///
    /// Sends render actions to this offscreen render target
    ///
    fn render<ActionIter: IntoIterator<Item=RenderAction>>(&mut self, actions: ActionIter) {
        unsafe {
            panic_on_gl_error("Preparing to render to offscreen buffer");

            // Remember the active render target
            let mut previous_frame_buffer = 0;
            gl::GetIntegerv(gl::DRAW_FRAMEBUFFER_BINDING, &mut previous_frame_buffer);

            // Render to the main render target
            gl::BindFramebuffer(gl::FRAMEBUFFER, *self.main_render_target);
            self.renderer.prepare_to_render_to_active_framebuffer(self.width, self.height);

            // Perform the rendering actions
            self.renderer.render(actions);
            gl::Flush();

            // Reset to the original render target
            gl::BindFramebuffer(gl::FRAMEBUFFER, previous_frame_buffer as gl::types::GLuint);

            panic_on_gl_error("After rendering to offscreen buffer");
        }
    }

    ///
    /// Consumes this render target and returns the realized pixels as a byte array
    ///
    fn realize(self) -> Vec<u8> {
        // Allocate space for the image
        let size_bytes  = self.width * self.height * 4;
        let mut pixels  = vec![0; size_bytes];

        // Read the image from the main texture into the pixel array
        let texture     = self.main_render_target.texture().expect("Offscreen texture");
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, *texture);
            gl::GetTexImage(gl::TEXTURE_2D, 0, gl::RGBA, gl::UNSIGNED_BYTE, pixels.as_mut_ptr() as *mut c_void);

            panic_on_gl_error("Read offscreen texture");
        }

        pixels
    }
}
