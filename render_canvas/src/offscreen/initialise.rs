use super::offscreen_trait::*;

use flo_canvas::*;
use flo_render::*;
use futures::prelude::*;
use futures::future::{LocalBoxFuture};

use std::cell::*;
use std::rc::*;

///
/// Rust's built-in boxing can't deal with the drawing context type. This gives us a way to dynamically dispatch to different types of drawing context,
/// allowing initialise_offscreen rendering to pick between them.
///
struct BoxedDrawingContext {
    create_drawing_target: Box<dyn FnMut(usize, usize) -> BoxedDrawingTarget>,
}

///
/// Dynamic dispatch for the drawing target
///
struct BoxedDrawingTarget {
    draw_actions: Box<dyn FnMut(Box<dyn Iterator<Item=Draw>>) -> LocalBoxFuture<'static, ()>>,
    realize: Box<dyn Fn() -> Vec<u8>>,
}

///
/// Creates a drawing context for rendering 2D graphics to an off-screen buffer
///
pub fn initialize_offscreen_rendering() -> Result<impl OffscreenDrawingContext, RenderInitError> {
    // We use the error from the last approach that we try
    let mut error = RenderInitError::ApiNotAvailable;

    // Use WGPU rendering for preference if multiple rendering engines are available
    #[cfg(feature="render-wgpu")]
    {
        use ::desync::*;
        use futures::prelude::*;
        use super::hardware::*;

        let wgpu_rendering = Desync::new(()).future_desync(|_| async { wgpu_initialize_offscreen_rendering().await }.boxed()).sync().unwrap();

        match wgpu_rendering {
            Ok(render_target) => {
                return Ok(BoxedDrawingContext::new(HardwareDrawingContext::from(render_target)));
            }

            Err(err) => { error = err; }
        }
    }

    // Metal hardware renderer
    #[cfg(feature="osx-metal")]
    {
        use ::desync::*;
        use futures::prelude::*;
        use super::hardware::*;

        let metal_rendering = metal_initialize_offscreen_rendering();

        match metal_rendering {
            Ok(render_target) => {
                return Ok(BoxedDrawingContext::new(HardwareDrawingContext::from(render_target)));
            }

            Err(err) => { error = err; }
        }
    }

    // OpenGL hardware renderer
    #[cfg(feature="opengl")]
    {
        use ::desync::*;
        use futures::prelude::*;
        use super::hardware::*;

        let metal_rendering = opengl_initialize_offscreen_rendering();

        match metal_rendering {
            Ok(render_target) => {
                return Ok(BoxedDrawingContext::new(HardwareDrawingContext::from(render_target)));
            }

            Err(err) => { error = err; }
        }
    }

    // Software renderer acts as a general fallback
    #[cfg(feature="render-software")]
    {
        use super::software::*;

        // Error won't be generated for the software renderer at this point (this suppresses various warnings)
        error = error;
        let _error = error;

        let software_rendering = SoftwareDrawingContext::default();
        return Ok(BoxedDrawingContext::new(software_rendering));
    }

    #[cfg(not(feature="render-software"))]
    // Return the error
    if false {
        // This never happens, but we want to specify the return tupe if there are no rendering engines defined at all
        Ok(BoxedDrawingContext { create_drawing_target: Box::new(|_, _| { panic!() }) })
    } else {
        Err(error)
    }
}

impl OffscreenDrawingContext for BoxedDrawingContext {
    type DrawingTarget = BoxedDrawingTarget;

    fn create_drawing_target(&mut self, width: usize, height: usize) -> Self::DrawingTarget {
        (self.create_drawing_target)(width, height)
    }
}

impl OffscreenDrawingTarget for BoxedDrawingTarget {
    #[inline]
    async fn draw_actions(&mut self, actions: impl 'static + IntoIterator<Item=Draw>) {
        (self.draw_actions)(Box::new(actions.into_iter())).await;
    }

    #[inline]
    fn realize(self) -> Vec<u8> {
        (self.realize)()
    }
}

impl BoxedDrawingContext {
    ///
    /// Boxes a render context
    ///
    pub fn new(render_context: impl 'static + OffscreenDrawingContext) -> Self {
        let mut render_context = render_context;
        let create_drawing_target = Box::new(move |width, height| {
            // We share the render target between the two functions (the render target is consumed by realize())
            // We use Rc<> here as drawing contexts can't always be shared between threads
            let render_target = render_context.create_drawing_target(width, height);
            let render_target = Rc::new(RefCell::new(Some(render_target)));
            
            // Add a callback for the draw actions
            let draw_actions_render_target = render_target.clone();
            let draw_actions = Box::new(move |draw_iterator: Box<dyn Iterator<Item=Draw>>| {
                let draw_actions_render_target = draw_actions_render_target.clone();
                async move {
                    let mut render_target   = draw_actions_render_target.borrow_mut();
                    let render_target       = (*render_target).as_mut().unwrap();

                    render_target.draw_actions(draw_iterator).await;
                }.boxed_local()
            });

            // Realizing the render target also consumes it
            let realize_render_target = render_target.clone();
            let realize = Box::new(move || {
                let mut render_target   = realize_render_target.borrow_mut();
                let render_target       = (*render_target).take().unwrap();

                render_target.realize()
            });

            // Create the drawing target
            BoxedDrawingTarget {
                draw_actions,
                realize
            }
        });

        BoxedDrawingContext {
            create_drawing_target
        }
    }
}