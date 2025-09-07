use super::canvas_drawing::*;

use crate::edgeplan::*;
use crate::pixel::*;
use crate::render::*;
use crate::scanplan::*;

use std::marker::{PhantomData};
use std::ops::{Range};

use std::sync::*;

///
/// Renders sections of a canvas drawing
///
pub struct CanvasDrawingRegionRenderer<TScanPlanner, TLineRenderer, TPixel, const N: usize>
where
    TPixel:         'static + Send + Sync + Pixel<N>,
    TScanPlanner:   ScanPlanner,
    TLineRenderer:  Renderer<Region=ScanlineRenderRegion, Source=ScanlinePlan>,
{
    /// Half the height in pixels to render the region at
    half_height: f64,

    /// 1/half_height
    half_height_recip: f64,

    /// The inverse of the amount to scale the region by (0.5 = double size)
    scale: (f64, f64),

    /// The translation to apply to the region
    translation: (f64, f64),

    /// The scan planner to use
    scan_planner: TScanPlanner,

    /// The scanline renderer
    line_renderer: TLineRenderer,

    /// Type of a pixel
    pixel: PhantomData<TPixel>,
}

impl<TScanPlanner, TLineRenderer, TPixel, const N: usize> CanvasDrawingRegionRenderer<TScanPlanner, TLineRenderer, TPixel, N>
where
    TPixel:         'static + Send + Sync + Pixel<N>,
    TScanPlanner:   ScanPlanner,
    TLineRenderer:  Renderer<Region=ScanlineRenderRegion, Source=ScanlinePlan>,
{
    ///
    /// Creates a new renderer that will render for a viewport with the specified height
    ///
    pub fn new(planner: TScanPlanner, line_renderer: TLineRenderer, height: usize) -> Self {
        CanvasDrawingRegionRenderer { 
            half_height:        (height as f64)/2.0, 
            half_height_recip:  1.0/((height as f64)/2.0),
            scale:              (1.0, 1.0),
            translation:        (0.0, 0.0),
            scan_planner:       planner,
            line_renderer:      line_renderer,
            pixel:              PhantomData,
        }
    }

    ///
    /// Converts an x-range in pixels to canvas coordinates
    ///
    /// We use a range of -1, 1 for the height regardless of the size of the render
    /// target, but the width can vary depending on the aspect ratio of the render
    /// target.
    ///
    #[inline]
    fn convert_width(&self, width: usize) -> Range<f64> {
        let width       = width as f64;
        let half_width  = width/2.0;
        let ratio       = half_width / self.half_height;

        let left        = (-ratio + self.translation.0) * self.scale.0;
        let right       = (ratio + self.translation.0) * self.scale.0;

        left..right
    }

    ///
    /// Converts y positions to the -1, 1 range we need 
    ///
    #[inline]
    fn convert_y_positions(&self, y_positions: &[f64]) -> Vec<f64> {
        let mut result = Vec::with_capacity(y_positions.len());
        result.extend(y_positions.iter()
            .map(|ypos| (ypos * self.half_height_recip - 1.0 + self.translation.1) * self.scale.1));

        result
    }

    ///
    /// Sets the viewport so that the corners are at the specified positions in canvas coordinates
    /// (when rendering on a canvas with the specified width)
    ///
    pub fn viewport_fit_exact(&mut self, source: &CanvasDrawing<TPixel, N>, min: (f64, f64), max: (f64, f64), width: f64) {
        // TODO: this won't flip the coordinates if min > max

        // The active transform converts from canvas pixels to the -1, 1 range that we use for rendering
        let transform = source.active_transform();

        // In the horizontal axis, we're rendering from -ratio to ratio
        let width       = width as f64;
        let half_width  = width/2.0;
        let ratio       = half_width / self.half_height;

        // Convert the coordinates from the canvas coordinate system to the universal one
        let (min_x, min_y) = transform.transform_point(min.0 as _, min.1 as _);
        let (max_x, max_y) = transform.transform_point(max.0 as _, max.1 as _);

        let (min_x, max_x) = (min_x.min(max_x), min_x.max(max_x));
        let (min_y, max_y) = (min_y.min(max_y), min_y.max(max_y));

        // Scale is the ratio between the width and the height (remember that this is the inverse scale)
        let scale_x = (max_x-min_x)/2.0;
        let scale_y = (max_y-min_y)/2.0;
        let scale_x = scale_x / (ratio as f32);

        // Set up the translation so that -ratio,-1 is mapped to min_x, min_y
        let translate_x = min_x/scale_x + (ratio as f32);
        let translate_y = min_y/scale_y - -1.0;

        // Store the results so they affect the next rendering
        self.translation    = (translate_x as _, translate_y as _);
        self.scale          = (scale_x as _, scale_y as _);
    }
}

impl<TScanPlanner, TLineRenderer, TPixel, const N: usize> Renderer for CanvasDrawingRegionRenderer<TScanPlanner, TLineRenderer, TPixel, N>
where
    TPixel:         'static + Send + Sync + Pixel<N>,
    TScanPlanner:   ScanPlanner<Edge=Arc<dyn EdgeDescriptor>>,
    TLineRenderer:  Renderer<Region=ScanlineRenderRegion, Source=ScanlinePlan, Dest=[TPixel]>,
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
            .map(|ypos| {
                let mut background_plan = ScanlinePlan::default();
                background_plan.push_next_range(0.0..(region.width as f64), true, [PixelProgramPlan::Run(source.background)]);

                (ypos, background_plan)
            })
            .collect::<Vec<_>>();
        let mut layer_scanlines = vec![(0.0, ScanlinePlan::default()); y_positions.len()];

        for layer_handle in source.ordered_layers.iter().copied() {
            if let Some(layer) = source.layers.get(layer_handle.0) {
                // Plan this layer (note that the x-range will be something like -1..1 so the scan planner must support this)
                if layer.alpha > 0.0 {
                    self.scan_planner.plan_scanlines(&layer.edges, &transform, &y_positions, x_range.clone(), &mut layer_scanlines);
                }

                // Combine the layer with the scanlines we're planning
                if layer.blend_mode == AlphaOperation::SourceOver && layer.alpha >= 1.0 {
                    // Source over, full transparency: just overlay the layers
                    scanlines.iter_mut()
                        .zip(layer_scanlines.iter())
                        .for_each(|((_, scanline), (_, layer_scanline))| {
                            scanline.merge(&layer_scanline, |src, dst, is_opaque| {
                                if is_opaque {
                                    src.clear();
                                }

                                src.extend(dst);
                            })
                        })
                } else if layer.alpha > 0.0 && layer.blend_mode == AlphaOperation::SourceOver {
                    // Blend the layers together using the source over operation
                    let alpha = layer.alpha as f32;

                    scanlines.iter_mut()
                        .zip(layer_scanlines.iter())
                        .for_each(|((_, scanline), (_, layer_scanline))| {
                            scanline.merge(&layer_scanline, |src, dst, _is_opaque| {
                                src.push(PixelProgramPlan::StartBlend);
                                src.extend(dst);
                                src.push(PixelProgramPlan::SourceOver(alpha));
                            })
                        })
                } else if layer.alpha > 0.0 {
                    // Blend the layers together
                    let blend_mode  = layer.blend_mode;
                    let alpha       = layer.alpha as f32;

                    scanlines.iter_mut()
                        .zip(layer_scanlines.iter())
                        .for_each(|((_, scanline), (_, layer_scanline))| {
                            scanline.merge(&layer_scanline, |src, dst, _is_opaque| {
                                src.push(PixelProgramPlan::StartBlend);
                                src.extend(dst);
                                src.push(PixelProgramPlan::Blend(blend_mode, alpha));
                            })
                        })
                }
            }
        }

        // Pass the scanlines on to the line renderer to produce the final result
        let mut lines  = dest.chunks_exact_mut(region.width);
        let mut region = ScanlineRenderRegion {
            y_pos:      0.0,
            transform:  transform,
        };

        for idx in 0..y_positions.len() {
            let (ypos, scanline)    = &scanlines[idx];
            let line                = lines.next().unwrap();
            region.y_pos            = *ypos;

            self.line_renderer.render(&region, scanline, line);
        }
    }
}
