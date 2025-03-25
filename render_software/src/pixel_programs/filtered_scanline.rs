use crate::edgeplan::*;
use crate::filters::*;
use crate::pixel::*;
use crate::render::*;
use crate::scanplan::*;

use std::collections::{HashMap};
use std::ops::{Range};
use std::sync::*;
use std::marker::{PhantomData};

///
/// Applies a filter to the result of rendering a scanline program, scaling it to match the frame transform
///
/// The full rendering is only applied to the region of the scanlines that are actually rendered on screen
///
pub struct FilteredScanlineFrameProgram<TEdgeDescriptor, TPixel, TPlanner> {
    /// The filter to apply to the output of the scanline program
    pixel: PhantomData<Mutex<TPixel>>,

    // Edge descriptor data
    edge: PhantomData<TEdgeDescriptor>,

    /// The pixel planner
    planner: PhantomData<TPlanner>,
}

///
/// Applies a filter to the result of rendering a scanline program
///
/// The full rendering is only applied to the region of the scanlines that are actually rendered on screen
///
pub struct FilteredScanlineProgram<TEdgeDescriptor, TFilter, TPlanner> {
    /// The filter to apply to the output of the scanline program
    filter: PhantomData<TFilter>,

    // Edge descriptor data
    edge: PhantomData<TEdgeDescriptor>,

    /// The pixel planner
    planner: TPlanner,
}

///
/// Data that can be used to run a basic sprite program
///
pub struct FilteredScanlineData<TEdgeDescriptor, TFilter>
where
    TEdgeDescriptor: EdgeDescriptor,
{
    /// The edges that will be used to generate the scanplan for this program
    edges: Arc<EdgePlan<TEdgeDescriptor>>,

    /// The scaling to apply to coordinates supplied to the edge plan
    scale: (f64, f64),

    /// The translation to apply to coordinates supplied to the edge plan
    translate: (f64, f64),

    /// The scanline plan for each y-position (updated for new scanlines)
    scanlines: RwLock<HashMap<u64, Arc<ScanlinePlan>>>,

    /// The filter to apply to the pixels generated from the scanlines
    filter: TFilter,
}

impl<TEdgeDescriptor, TFilter, TPlanner> Default for FilteredScanlineProgram<TEdgeDescriptor, TFilter, TPlanner> 
where
    TFilter::Pixel:     'static + AlphaBlend + Copy + Clone + Default,
    TFilter:            Send + Sync + PixelFilter,
    TEdgeDescriptor:    EdgeDescriptor,
    TPlanner:           Send + Sync + Default + ScanPlanner<Edge=TEdgeDescriptor>,
{
    fn default() -> Self {
        Self {
            filter:     PhantomData,
            edge:       PhantomData,
            planner:    TPlanner::default(),
        }
    }
}

impl<TEdgeDescriptor, TPixel, TPlanner> Default for FilteredScanlineFrameProgram<TEdgeDescriptor, TPixel, TPlanner> 
where
    TPixel:             'static + AlphaBlend + Send + Copy + Clone + Default,
    TEdgeDescriptor:    EdgeDescriptor,
    TPlanner:           Send + Sync + Default + ScanPlanner<Edge=TEdgeDescriptor>,
{
    fn default() -> Self {
        Self {
            pixel:      PhantomData,
            edge:       PhantomData,
            planner:    PhantomData,
        }
    }
}

impl<TEdgeDescriptor, TFilter> FilteredScanlineData<TEdgeDescriptor, TFilter>
where
    TEdgeDescriptor:    EdgeDescriptor,
    TFilter:            Send + Sync + PixelFilter,
{
    ///
    /// Creates a new instance of the data for the basic sprite pixel program
    ///
    pub fn new(edges: Arc<EdgePlan<TEdgeDescriptor>>, scale: (f64, f64), translate: (f64, f64), filter: TFilter) -> Self {
        let scanlines = RwLock::new(HashMap::new());

        FilteredScanlineData { edges, scale, translate, scanlines, filter }
    }
}

impl<TEdgeDescriptor, TPixel, TPlanner> PixelProgramForFrame for FilteredScanlineFrameProgram<TEdgeDescriptor, TPixel, TPlanner>
where
    TPixel:             'static + AlphaBlend + Send + Copy + Clone + Default,
    TEdgeDescriptor:    EdgeDescriptor,
    TPlanner:           Default + Send + Sync + ScanPlanner<Edge=TEdgeDescriptor>,
{
    type Program    = FilteredScanlineProgram<TEdgeDescriptor, Arc<dyn Send + Sync + PixelFilter<Pixel=TPixel>>, TPlanner>;
    type FrameData  = FilteredScanlineData<TEdgeDescriptor, Arc<dyn Send + Sync + PixelFilter<Pixel=TPixel>>>;

    fn program_for_frame(&self, pixel_size: PixelSize, program_data: &Arc<Self::FrameData>) -> (Self::Program, <Self::Program as PixelProgram>::ProgramData) {
        let program = FilteredScanlineProgram::default();

        if pixel_size.0 == 1.0 {
            // Pixel is the same size
            (program, Arc::clone(program_data))
        } else {
            // Adjust the filter
            if let Some(new_filter) = program_data.filter.with_scale(1.0/pixel_size.0, 1.0/pixel_size.0) {
                let new_program_data = FilteredScanlineData {
                    edges:      program_data.edges.clone(),
                    scale:      program_data.scale,
                    translate:  program_data.translate,
                    scanlines:  RwLock::new(HashMap::new()),
                    filter:     new_filter,
                };

                (program, Arc::new(new_program_data))
            } else {
                // Filter isn't changed
                (program, Arc::clone(program_data))
            }
        }
    }
}

impl<TEdgeDescriptor, TFilter, TPlanner> PixelProgram for FilteredScanlineProgram<TEdgeDescriptor, TFilter, TPlanner>
where
    TFilter::Pixel:     'static + AlphaBlend + Copy + Clone + Default,
    TFilter:            Send + Sync + PixelFilter,
    TEdgeDescriptor:    EdgeDescriptor,
    TPlanner:           Send + Sync + ScanPlanner<Edge=TEdgeDescriptor>,
{
    type Pixel          = TFilter::Pixel;
    type ProgramData    = Arc<FilteredScanlineData<TEdgeDescriptor, TFilter>>;

    #[inline]
    fn draw_pixels(&self, data_cache: &PixelProgramRenderCache<Self::Pixel>, target: &mut [Self::Pixel], pixel_range: Range<i32>, x_transform: &ScanlineTransform, y_pos: f64, data: &Self::ProgramData) {
        use std::mem;

        // TODO: a performance observation is that the version of this that only ever generated a single line was twice as fast as the version that generates the extra before/after lines
        // (slowdowns due to the extra iterators, collections, etc?)

        // Fetch the 'buffer' area of extra pixels to render for the filter
        let (before_x, after_x) = data.filter.extra_columns();
        let (before_y, after_y) = data.filter.input_lines();

        // Use the scale and translation factors to get the transform for the edges
        let scan_ypos       = y_pos * data.scale.1 + data.translate.1;
        let scan_transform  = x_transform.transform(data.scale.0, data.translate.0);
        let pixel_ypos      = x_transform.source_x_to_pixels(y_pos);

        // Try to retrieve the scanline, or plan a new one if needed
        // TODO: reset scanlines if the x-transform or the width has changed (maybe also if the render height is changed)
        let render_scanlines = (-(before_y as i64)..=(after_y as i64))
            .map(|y_offset| {
                // Assume that pixels are square and use the x-transform to convert y positions. Note that the offset is in 'canvas' pixels and not the transformed 'sprite' pixels
                let pixel_ypos  = pixel_ypos + (y_offset as f64);
                let ypos        = x_transform.fractional_pixel_x_to_source_x(pixel_ypos);
                let scan_ypos   = ypos * data.scale.1 + data.translate.1;

                // TODO: 'to_bits()' here will result in cache misses (slower perf, higher memory usage)
                let scanlines = data.scanlines.read().unwrap();
                if let Some(existing_scanline) = scanlines.get(&pixel_ypos.to_bits()) {
                    // Re-use the previously calculated scanline
                    Arc::clone(existing_scanline)
                } else {
                    // Cache a new scanline to re-use
                    mem::drop(scanlines);

                    // Calculate the transform for the sprite region
                    let scan_xrange = scan_transform.pixel_x_to_source_x(-(before_x as i32))..scan_transform.pixel_x_to_source_x(x_transform.width_in_pixels() as i32 + after_x as i32);

                    // Plan the rendering for the sprite
                    let mut new_scanline = [(scan_ypos, ScanlinePlan::default())];
                    self.planner.plan_scanlines(&*data.edges, &scan_transform, &[scan_ypos], scan_xrange, &mut new_scanline);

                    // Store as the new cached value
                    let mut scanlines = data.scanlines.write().unwrap();

                    let mut new_plan = ScanlinePlan::default();
                    mem::swap(&mut new_scanline[0].1, &mut new_plan);
                    let new_scanline = Arc::new(new_plan);

                    scanlines.insert(pixel_ypos.to_bits(), new_scanline.clone());

                    new_scanline
                }
            });

        // Clip the plan against the x-region that's being rendered (so we don't render any more pixels than we actually need)
        let x_start             = pixel_range.start as f64 - before_x as f64;
        let x_end               = pixel_range.end as f64 + after_x as f64;
        let x_range             = x_start..x_end;
        let render_scanlines    = render_scanlines.map(|scanline| scanline.clip(x_range.clone(), x_start));

        // Render the scanlines into their own buffers
        let rendered_scanlines = (-(before_y as i64)..=(after_y as i64)).zip(render_scanlines)
            .map(|(y_offset, scanline)| {
                let scan_ypos = scan_ypos + (y_offset as f64);

                let region              = ScanlineRenderRegion { y_pos: scan_ypos, transform: scan_transform };
                let mut scanline_buffer = vec![<TFilter as PixelFilter>::Pixel::default(); pixel_range.len() + (before_x + after_x)];
                data_cache.render(&region, &scanline, &mut scanline_buffer);

                scanline_buffer
            })
            .collect::<Vec<_>>();

        let scanline_refs = rendered_scanlines.iter()
            .map(|scanline| {
                let scanline: &[TFilter::Pixel] = scanline;
                scanline
            })
            .collect::<Vec<_>>();

        // Apply the filter to generate the final result
        let mut filter_result   = vec![<TFilter as PixelFilter>::Pixel::default(); pixel_range.len()];
        let filter_ypos         = scan_transform.source_x_to_pixels(scan_ypos) - scan_transform.source_x_to_pixels(-1.0);

        data.filter.filter_line(filter_ypos as usize, &scanline_refs, &mut filter_result);

        for (src, tgt) in filter_result[0..pixel_range.len()].iter().zip(target[(pixel_range.start as usize)..(pixel_range.end as usize)].iter_mut()) {
            *tgt = src.source_over(*tgt);
        }
    }
}
