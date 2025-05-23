use super::offscreen_trait::*;
use crate::canvas_renderer::*;

use flo_render::*;

use futures::prelude::*;

///
/// A hardware drawing context uses a flo_render context to render drawing instructions (these are hardware accellerated by a GPU)
///
pub struct HardwareDrawingContext<TRenderContext> 
where 
    TRenderContext: OffscreenRenderContext,
{
    render_context: TRenderContext
}

pub struct HardwareDrawingTarget<TRenderTarget>
where 
    TRenderTarget: OffscreenRenderTarget,
{
    renderer:       CanvasRenderer,
    render_target:  TRenderTarget,
}

impl<TRenderContext> From<TRenderContext> for HardwareDrawingContext<TRenderContext>
where 
    TRenderContext: OffscreenRenderContext,
{
    ///
    /// Creates a hardware drawing context from a render context
    ///
    #[inline]
    fn from(ctxt: TRenderContext) -> Self {
        HardwareDrawingContext { 
            render_context: ctxt,
        }
    }
}

impl<TRenderContext> OffscreenDrawingContext for HardwareDrawingContext<TRenderContext>
where 
    TRenderContext: OffscreenRenderContext,
{
    type DrawingTarget = HardwareDrawingTarget<TRenderContext::RenderTarget>;

    #[inline]
    fn create_drawing_target(&mut self, width: usize, height: usize) -> Self::DrawingTarget {
        let mut renderer = CanvasRenderer::new();

        // Prepare to render
        renderer.set_viewport(0.0..(width as f32), 0.0..(height as f32), width as f32, height as f32, 1.0);

        HardwareDrawingTarget {
            renderer:      renderer,
            render_target:  self.render_context.create_render_target(width, height),
        }
    }
}

impl<TRenderTarget> OffscreenDrawingTarget for HardwareDrawingTarget<TRenderTarget>
where 
    TRenderTarget: OffscreenRenderTarget,
{
    async fn draw_actions(&mut self, actions: impl 'static + IntoIterator<Item=flo_canvas::Draw>) {
        // Collect the actions
        let actions     = actions.into_iter().collect::<Vec<_>>();
        let rendering   = self.renderer.draw(actions.into_iter());
        let rendering   = rendering.collect::<Vec<_>>().await;

        // Send them to the renderer
        self.render_target.render(rendering);
    }

    fn realize(self) -> Vec<u8> {
        self.render_target.realize()
    }
}
