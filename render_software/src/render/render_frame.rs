use crate::draw::*;
use crate::pixel::*;
use crate::scanplan::*;

use super::render_source_trait::*;
use super::render_target_trait::*;
use super::scanline_renderer::*;

use flo_canvas as canvas;

///
/// Renders and entire frame to a render target from a render source
///
pub fn render_frame_with_planner<'a, TScanPlanner, TProgramRunner, TSource, TTarget>(scan_planner: TScanPlanner, program_runner: TProgramRunner, source: &TSource, target: &mut TTarget)
where
    TScanPlanner:           ScanPlanner,
    TProgramRunner:         PixelProgramRunner,
    TProgramRunner::TPixel: 'static,
    TSource:                RenderSource<TScanPlanner, TProgramRunner>,
    TTarget:                RenderTarget<TProgramRunner::TPixel>,
{
    let region_renderer = TSource::create_region_renderer(scan_planner, program_runner);
    target.render(region_renderer, source);
}

///
/// Renders a set of drawing instructions to a target using the default settings
///
pub fn render_drawing<TTarget>(target: &mut TTarget, drawing: impl IntoIterator<Item=canvas::Draw>) 
where
    TTarget: RenderTarget<F32LinearPixel>,
{
    // Prepare a canvas drawing
    let mut canvas_drawing = CanvasDrawing::<F32LinearPixel, 4>::empty();
    canvas_drawing.draw(drawing);

    let renderer = CanvasDrawingRegionRenderer::new(PixelScanPlanner::default(), ScanlineRenderer::new(canvas_drawing.program_runner()), target.height());
    target.render(renderer, &canvas_drawing);
}
