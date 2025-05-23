use super::offscreen_trait::*;

use flo_canvas::*;
use flo_render_software::draw::*;
use flo_render_software::pixel::*;
use flo_render_software::render::*;
use flo_render_software::scanplan::*;

///
/// Offscreen drawing context that uses the software renderer
///
pub struct SoftwareDrawingContext {
    scale: f64,
    gamma: f64,
}

///
/// Sends drawing commands to a buffer using the software renderer
///
pub struct SoftwareDrawingTarget {
    canvas: CanvasDrawing<F32LinearPixel, 4>,
    width:  usize,
    height: usize,
    gamma:  f64,
}

impl Default for SoftwareDrawingContext {
    fn default() -> Self {
        SoftwareDrawingContext {
            scale: 1.0,
            gamma: 2.2,
        }
    }
}

impl SoftwareDrawingContext {
    ///
    /// Creates a new software drawing context. 
    ///
    /// `scale` sets the pixel scaling - this affects the size of a pixel for calls like `line_width_pixels`, and is used for cases where the DPI
    /// of the output is higher than the input program is assuming. Defaults to 1.0
    ///
    /// `gamma` sets the gamma factor for the output. This is usually 2.2 for output to a monitor.
    ///
    pub fn new(scale: f64, gamma: f64) -> Self {
        SoftwareDrawingContext { scale, gamma }
    }
}

impl OffscreenDrawingContext for SoftwareDrawingContext {
    type DrawingTarget = SoftwareDrawingTarget;

    fn create_drawing_target(&mut self, width: usize, height: usize) -> Self::DrawingTarget {
        let gamma       = self.gamma;
        let mut canvas  = CanvasDrawing::empty();
        canvas.set_pixel_height((height as f64) / self.scale);
        canvas.set_base_transform(Transform2D::identity());

        SoftwareDrawingTarget { canvas, width, height, gamma }
    }
}

impl OffscreenDrawingTarget for SoftwareDrawingTarget {
    #[inline]
    async fn draw_actions(&mut self, actions: impl 'static + IntoIterator<Item=flo_canvas::Draw>) {
        self.canvas.draw(actions);
    }

    fn realize(self) -> Vec<u8> {
        // Create the frame to render to
        let mut frame   = vec![0u8; self.width*self.height*4];
        let mut rgba    = FrameU8Rgba::from_bytes(self.width, self.height, self.gamma, &mut frame).unwrap();

        // Render the canvas to the buffer
        let renderer = CanvasDrawingRegionRenderer::new(ShardScanPlanner::default(), ScanlineRenderer::new(self.canvas.program_runner(self.height as _)), self.height);
        rgba.render(renderer, &self.canvas);

        frame
    }
}