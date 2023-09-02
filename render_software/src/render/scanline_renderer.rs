use super::renderer::*;

use crate::pixel::*;
use crate::scanplan::*;
use crate::scanplan::buffer_stack::*;

///
/// Specifies the y-pos and x transformation to use with a scanline renderer
///
#[derive(Copy, Clone)]
pub struct ScanlineRenderRegion {
    /// The y position of this scanline
    pub y_pos: f64,

    /// The transform to apply to get the source range from the pixels
    pub transform: ScanlineTransform,
}

///
/// Renders a ScanPlan using a particular pixel type
///
pub struct ScanlineRenderer<TProgramRunner>
where
    TProgramRunner:         PixelProgramRunner,
    TProgramRunner::TPixel: 'static + Send + Copy + AlphaBlend,
{
    program_data:   TProgramRunner,
}

impl<TProgramRunner> ScanlineRenderer<TProgramRunner>
where
    TProgramRunner:         PixelProgramRunner,
    TProgramRunner::TPixel: 'static + Send + Copy + AlphaBlend,
{
    ///
    /// Creates a new scanline renderer
    ///
    #[inline]
    pub fn new(data_cache: TProgramRunner) -> Self {
        ScanlineRenderer {
            program_data:   data_cache,
        }
    }
}

impl<TProgramRunner> Renderer for ScanlineRenderer<TProgramRunner>
where
    TProgramRunner:         PixelProgramRunner,
    TProgramRunner::TPixel: 'static + Send + Copy + AlphaBlend,
{
    type Region = ScanlineRenderRegion;
    type Source = ScanlinePlan;
    type Dest   = [TProgramRunner::TPixel];

    ///
    /// Renders a `ScanlinePlan` to a buffer of pixels (which should match the length of the plan)
    ///
    /// The y-position here is relayed to the pixel program when generating the actual pixels for the scanline
    ///
    fn render(&self, region: &Self::Region, source: &Self::Source, dest: &mut Self::Dest) {
        let scanline        = dest;
        let spans           = source.spans();
        let y_pos           = region.y_pos;
        let transform       = &region.transform;

        // Check that the operations will fit over this scanline
        let start_pos   = spans.get(0).map(|span| span.x_range.start).unwrap_or(0.0);
        let end_pos     = spans.last().map(|span| span.x_range.end).unwrap_or(0.0);

        if (scanline.len() as f64) < end_pos.floor() {
            panic!("Scanline is too long (have {} pixels, but want to write {})", end_pos, scanline.len());
        }

        if start_pos < 0.0 {
            panic!("Scanline starts before the start of the list of pixels (at {})", start_pos);
        }

        // The shadow stack keeps our copies of the scanline for blending operations, so we don't need to keep reallocating them
        let mut shadow_pixels = BufferStack::new(scanline);

        // Execute each span
        for span in spans.iter() {
            // Read the span and start iterating through the program IDs
            let x_range             = span.x_range.clone();
            let mut remaining_steps = span.plan.iter();
            let mut current_step    = remaining_steps.next().unwrap();

            loop {
                // Evaluate the current step of this span
                match current_step {
                    PixelProgramPlan::Run(data_id) => {
                        // Just run the program
                        let pixel_range = (x_range.start.floor() as _)..(x_range.end.ceil() as _);
                        self.program_data.run_program(*data_id, shadow_pixels.buffer(), pixel_range, transform, y_pos);
                    }

                    PixelProgramPlan::StartBlend => {
                        // Add a new copy of the pixels to the shadow stack
                        shadow_pixels.push_entry((x_range.start as _)..(x_range.end as _));
                    },

                    PixelProgramPlan::Blend(factor) => {
                        let factor = *factor as f64;

                        // Can skip the factor multiplication step if the blend factor is 1.0 (which should be fairly common)
                        if factor == 1.0 {
                            shadow_pixels.pop_entry(|src, dst| {
                                for x in (x_range.start as usize)..(x_range.end as usize) {
                                    dst[x] = src[x].source_over(dst[x]);
                                }
                            });
                        } else {
                            shadow_pixels.pop_entry(|src, dst| {
                                for x in (x_range.start as usize)..(x_range.end as usize) {
                                    dst[x] = (src[x].multiply_alpha(factor)).source_over(dst[x]);
                                }
                            });
                        }
                    },

                    PixelProgramPlan::LinearBlend(start, end) => {
                        // Change the alpha factor across the range of the blend
                        let x_range     = (x_range.start as usize)..(x_range.end as usize);
                        let start       = *start as f64;
                        let end         = *end as f64;
                        let multiplier  = (end-start)/(x_range.len() as f64);

                        shadow_pixels.pop_entry(|src, dst| {
                            let start_x = x_range.start;

                            for x in x_range {
                                let pos     = (x-start_x) as f64;
                                let factor  = start + pos * multiplier;

                                dst[x] = (src[x].multiply_alpha(factor)).source_over(dst[x]);
                            }
                        });
                    }
                }

                // Move to the next step
                if let Some(next_step) = remaining_steps.next() {
                    current_step = next_step;
                } else {
                    break;
                }
            }
        }
    }
}
