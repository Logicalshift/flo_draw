use super::pixel_filter_trait::*;
use crate::pixel::*;

use std::sync::*;

///
/// The mask filter multiplies the output pixels by the contents of a mask texture
///
pub struct CombinedFilter<TPixel, const N: usize>
where
    TPixel: Pixel<N>,
{
    /// The filters that are being copmbined
    filters: Vec<Arc<dyn Send + Sync + PixelFilter<Pixel=TPixel>>>,
}

impl<TPixel, const N: usize> CombinedFilter<TPixel, N>
where
    TPixel: Pixel<N>,
{
    ///
    /// Creates a combined filter from a set of input filters (all the input filters will be applied to the result)
    ///
    pub fn from_filters(filters: impl IntoIterator<Item=Arc<dyn Send + Sync + PixelFilter<Pixel=TPixel>>>) -> Self {
        CombinedFilter {
            filters: filters.into_iter().collect(),
        }
    }
}

impl<TPixel, const N: usize> PixelFilter for CombinedFilter<TPixel, N>
where
    TPixel: 'static + Pixel<N>,
{
    type Pixel = TPixel;

    fn with_scale(&self, x_scale: f64, y_scale: f64) -> Option<Arc<dyn Send + Sync + PixelFilter<Pixel=Self::Pixel>>> {
        let new_filters = self.filters.iter()
            .map(|old_filter| {
                old_filter.with_scale(x_scale, y_scale).unwrap_or_else(|| Arc::clone(old_filter))
            })
            .collect();

        Some(Arc::new(Self {
            filters: new_filters
        }))
    }

    fn input_lines(&self) -> (usize, usize) {
        // The combined value is found by summing the requested values from all of the filters we're combining
        let mut top     = 0;
        let mut bottom  = 0;

        for filter in self.filters.iter() {
            let (filter_top, filter_bottom) = filter.input_lines();
            top     = filter_top + top;
            bottom  = filter_bottom + bottom;
        }

        (top, bottom)
    }

    fn extra_columns(&self) -> (usize, usize) {
        // The combined value is found by summing the requested values from all of the filters we're combining
        let mut left    = 0;
        let mut right   = 0;

        for filter in self.filters.iter() {
            let (filter_left, filter_right) = filter.extra_columns();
            left    = filter_left + left;
            right   = filter_right + right;
        }

        (left, right)
    }

    fn filter_line(&self, y_pos: usize, input_lines: &[&[Self::Pixel]], output_line: &mut [Self::Pixel]) {
        use std::mem;

        if self.filters.len() == 0 {
            // Edge case: no filters = copy the input to the output
            for (input, output) in input_lines[0].iter().zip(output_line.iter_mut()) {
                *output = *input;
            }
        } else if self.filters.len() == 1 {
            // Edge case: just call the first filter directly
            self.filters[0].filter_line(y_pos, input_lines, output_line)
        } else {
            // Apply each filter in turn to generate the input for the next filter along
            let (first_left, first_right)   = self.filters[0].extra_columns();
            let (first_top, first_bottom)   = self.filters[0].input_lines();

            // The width and height here are the number of input pixels for the next filter
            let mut width                   = input_lines[0].len();
            let mut height                  = input_lines.len();

            // Generate enough output lines to fill in the next filter in the seqeunce (we'll end up with one at the end)
            let mut output      = vec![vec![TPixel::default(); width - first_left - first_right]; height - first_top - first_bottom];

            // The next output becomes the input for the next level of the filter
            let mut next_output = output.clone();

            // The next input are references to either input_lines or next_output
            let mut next_input  = input_lines.iter().map(|pixels| *pixels).collect::<Vec<&[Self::Pixel]>>();

            // Middle filters all process from output to output
            for filter in self.filters.iter().take(self.filters.len()-1) {
                // Number of pixels that will be trimmed from the input
                let (left, right)   = filter.extra_columns();
                let (top, bottom)   = filter.input_lines();

                // Filter each line into the output
                for output_line in 0..(height-bottom-top) {
                    filter.filter_line(y_pos + output_line, 
                        &next_input[output_line..(output_line+1+top+bottom)], 
                        &mut output[output_line][0..(width-left-right)]);
                }

                // Width and height are updated for the next iteration
                width -= left+right;
                height -= top+bottom;

                // Swap the output and the next output so we'll write to a new buffer
                mem::swap(&mut output, &mut next_output);

                // Regenerate the input lines from the next output
                next_input = (0..height).map(|idx| &next_output[idx][0..width]).collect();
            }

            // Final filter writes to the output line
            self.filters.last().unwrap().filter_line(y_pos, &next_input[0..height], output_line);
        }
    }
}
