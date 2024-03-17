use crate::pixel::*;
use super::pixel_filter_trait::*;

use std::collections::*;
use std::sync::*;

///
/// Reads and filters the pixels from a texture
///
/// The result here is an iterator of the generated rows, using the texture reader pixel format
///
pub fn filter_texture<'a, TTexture, TPixel, const N: usize>(texture: &'a TTexture, filter: &'a impl PixelFilter<Pixel=TPixel>) -> impl 'a + Iterator<Item=Vec<TPixel>>
where
    TPixel:     'a + TextureReader<TTexture>,
    TPixel:     Pixel<N>,
    TTexture:   Send + Sync,
{
    // Read about the texture
    let (width, height) = TPixel::texture_size(texture);
    let width_pixels    = width as usize;
    let height_pixels   = height as usize;

    // The read_row function reads a single row from the texture to be filtered
    let read_row = (0..height_pixels).map(move |row_num| {
        TPixel::read_pixels_linear(texture, 0.0, 1.0, (1.0, 0.0), (0.0, row_num as f64), width_pixels)
    });

    apply_pixel_filter(width_pixels, read_row, filter)
}

///
/// Iterator that returns a 'window' of pixels that 'rolls' down a source set of pixels
///
struct RollingPixelCache<TPixel, TIterator> 
where
    TPixel:     Default,
    TIterator:  Iterator<Item=Vec<TPixel>>
{
    /// A blank line that we can re-use at the start and end of the cache
    blank_line: Arc<Vec<TPixel>>,

    /// The number of blank pixels to add to the start of each line
    padding_left: usize,

    /// The number of blank pixels to add to the end of each line
    padding_right: usize,

    /// The pixels in the current cache (initially filled with blank lines)
    cache: VecDeque<Arc<Vec<TPixel>>>,

    /// None when finished, or the iterator for the pixel lines
    iterator: Option<TIterator>,

    /// The number of blank lines remaining after the iterator has completed
    remaining_lines: usize,
}

impl<TPixel, TIterator> RollingPixelCache<TPixel, TIterator> 
where
    TPixel:     Default + Clone,
    TIterator:  Iterator<Item=Vec<TPixel>>,
{
    ///
    /// Creates a rolling pixel cache from an iterator
    ///
    pub fn from_iterator(iterator: TIterator, width: usize, add_above: usize, add_below: usize, add_left: usize, add_right: usize) -> Self {
        // If we're not adding blank lines, then use an empty vec, otherwise create a common blank line for the iterator to use
        let blank_line = if add_above + add_below > 0 { vec![TPixel::default(); width+add_left+add_right] } else { vec![] };
        let blank_line = Arc::new(blank_line);

        // Start by creating blank lines in the initial cache (up to add_above, plus one extra to be overwritten by the first iteration)
        let mut initial_cache = (0..=add_above).map(|_| Arc::clone(&blank_line)).collect::<VecDeque<_>>();

        // Read extra lines to put 'underneath'
        let mut iterator    = Some(iterator);
        let mut remaining   = add_below;

        for _ in 0..add_below {
            if let Some(mut pixels) = iterator.as_mut().and_then(|iterator| iterator.next()) {
                if add_left > 0 {
                    pixels.splice(0..0, (0..add_left).map(|_| TPixel::default()));
                }

                if add_right > 0 {
                    pixels.extend((0..add_right).map(|_| TPixel::default()));
                }

                initial_cache.push_back(Arc::new(pixels));
            } else {
                // Update the remaining count as we've run off the end of the iterator
                iterator    = None;
                remaining   = if remaining > 0 { remaining - 1 } else { 0 };

                // Add default pixels
                initial_cache.push_back(Arc::clone(&blank_line));
            }
        }

        // Fill up the final cache
        RollingPixelCache { 
            blank_line:         blank_line,
            cache:              initial_cache,
            iterator:           iterator, 
            remaining_lines:    remaining,
            padding_left:       add_left,
            padding_right:      add_right,
        }
    }
}

impl<TPixel, TIterator> Iterator for RollingPixelCache<TPixel, TIterator> 
where
    TPixel:     Default + Clone,
    TIterator:  Iterator<Item=Vec<TPixel>>,
{
    type Item = Box<[Arc<Vec<TPixel>>]>;

    fn next(&mut self) -> Option<Self::Item> {
        // Read the next line of pixels
        let pixels = if let Some(iterator) = self.iterator.as_mut() {
            if let Some(mut pixels) = iterator.next() {
                if self.padding_left > 0 {
                    pixels.splice(0..0, (0..self.padding_left).map(|_| TPixel::default()));
                }

                if self.padding_right > 0 {
                    pixels.extend((0..self.padding_right).map(|_| TPixel::default()));
                }

                Arc::new(pixels)
            } else {
                // Done with the iterator
                self.iterator = None;

                if self.remaining_lines > 0 {
                    // Iterator has run out, so return a blank line while there are remaining lines to consider
                    self.remaining_lines -= 1;
                    Arc::clone(&self.blank_line)
                } else {
                    // No more remaining lines
                    return None;
                }
            }
        } else {
            if self.remaining_lines > 0 {
                // Iterator has run out, so return a blank line while there are remaining lines to consider
                self.remaining_lines -= 1;
                Arc::clone(&self.blank_line)
            } else {
                // No more remaining lines
                return None;
            }
        };

        // Add to the cache
        self.cache.pop_front();
        self.cache.push_back(pixels);

        // Generate the result
        Some(self.cache.iter().cloned().collect())
    }
}

///
/// Given a function that can read a row of pixels at a time, applies a texture filter and returns an iterator 
///
pub fn apply_pixel_filter<'a, TPixel, const N: usize>(width: usize, read_row: impl 'a + Send + Sync + Iterator<Item=Vec<TPixel>>, filter: &'a impl PixelFilter<Pixel=TPixel>) -> impl 'a + Iterator<Item=Vec<TPixel>>
where
    TPixel: 'a + Pixel<N>,
{
    // We need to add extra clear pixels above and below the 
    let (add_above, add_below) = filter.input_lines();
    let (add_left, add_right)  = filter.extra_columns();

    // The pixel cache stores the lines we've read from the source that we're going to supply to the filter. If we're rendering extra lines, 
    // then we allow each line to be read a certain number of times
    RollingPixelCache::from_iterator(read_row, width, add_above, add_below, add_left, add_right)
        .map(move |pixel_rows| {
            // Filter the pixels
            let input_lines     = pixel_rows.iter().map(|pixels| &***pixels).collect::<Box<[_]>>();
            let mut output_line = vec![TPixel::default(); width];
            filter.filter_line(&*input_lines, &mut output_line);

            output_line
        })
}

