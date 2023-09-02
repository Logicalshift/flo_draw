use super::canvas_drawing::*;

use crate::edgeplan::*;
use crate::pixel::*;
use crate::render::*;
use crate::scanplan::*;

use std::marker::{PhantomData};
use std::ops::{Range};

///
/// Renders sections of a canvas drawing
///
pub struct CanvasDrawingRegionRenderer<TScanPlanner, TPixel, const N: usize>
where
    TPixel:         'static + Send + Sync + Pixel<N>,
    TScanPlanner:   ScanPlanner,
{
    /// Half the height in pixels to render the region at
    half_height: f64,

    /// 1/half_height
    half_height_recip: f64,

    /// The scan planner to use
    scan_planner: TScanPlanner,

    /// Type of a pixel
    pixel: PhantomData<TPixel>,
}

impl<TScanPlanner, TPixel, const N: usize> CanvasDrawingRegionRenderer<TScanPlanner, TPixel, N>
where
    TPixel:         'static + Send + Sync + Pixel<N>,
    TScanPlanner:   ScanPlanner,
{
    ///
    /// Creates a new renderer that will render for a viewport with the specified height
    ///
    pub fn new(&self, planner: TScanPlanner, height: usize) -> Self {
        CanvasDrawingRegionRenderer { 
            half_height:        (height as f64)/2.0, 
            half_height_recip:  1.0/((height as f64)/2.0),
            scan_planner:       planner,
            pixel:              PhantomData,
        }
    }

    ///
    /// Converts an x-range in pixels to canvas coordinates
    ///
    #[inline]
    fn convert_width(&self, width: usize) -> Range<f64> {
        let width       = width as f64;
        let half_width  = width/2.0;
        let ratio       = half_width / self.half_height;

        -ratio..ratio
    }

    ///
    /// Converts y positions to the -1, 1 range we need 
    ///
    #[inline]
    fn convert_y_positions(&self, y_positions: &[f64]) -> Vec<f64> {
        let mut result = Vec::with_capacity(y_positions.len());
        result.extend(y_positions.iter()
            .map(|ypos| ypos * self.half_height_recip - 1.0));

        result
    }
}

impl<TScanPlanner, TPixel, const N: usize> Renderer for CanvasDrawingRegionRenderer<TScanPlanner, TPixel, N>
where
    TPixel:         'static + Send + Sync + Pixel<N>,
    TScanPlanner:   ScanPlanner<Edge=Box<dyn EdgeDescriptor>>,
{
    type Region = RenderSlice;
    type Source = CanvasDrawing<TPixel, N>;
    type Dest   = [TPixel];

    fn render(&self, region: &RenderSlice, source: &CanvasDrawing<TPixel, N>, dest: &mut [TPixel]) {
        // Convert y positions to between -1 and 1 (canvas coordinates)
        let y_positions = self.convert_y_positions(&region.y_positions);
        let x_range     = self.convert_width(region.width);
        let transform   = ScanlineTransform::for_region(&x_range, region.width);

        // We need to plan scanlines for each layer, then merge them. The initial plan is just to fill the entire range with the background colour
        let mut scanlines       = y_positions.iter().copied()
            .map(|ypos| (ypos, ScanlinePlan::from_ordered_stacks(vec![ScanSpanStack::with_first_span(ScanSpan::opaque(0.0..(region.width as f64), source.background))])))
            .collect::<Vec<_>>();
        let mut layer_scanlines = vec![(0.0, ScanlinePlan::default()); y_positions.len()];

        for layer_handle in source.ordered_layers.iter().copied() {
            if let Some(layer) = source.layers.get(layer_handle.0) {
                // Plan this layer (note that the x-range will be something like -1..1 so the scan planner must support this)
                self.scan_planner.plan_scanlines(&layer.edges, &transform, &y_positions, x_range.clone(), &mut layer_scanlines);

                // TODO: Combine the layer with the scanlines we're planning
            }
        }

        // TODO: Convert the scanlines back to render coordinates

        // TODO: Pass the scanlines on to the line renderer to produce the final result

        todo!()
    }
}
