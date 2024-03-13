use crate::pixel::*;
use super::pixel_filter_trait::*;

use std::collections::*;
use std::sync::*;
use std::sync::atomic::{AtomicIsize, Ordering};

///
/// Reads and filters the pixels from a texture
///
/// The result here is an iterator of the generated rows, using the texture reader pixel format
///
pub fn filter_texture<'a, TTexture, TTextureReader, const N: usize>(texture: &'a TTexture, filter: &'a impl PixelFilter<Pixel=TTextureReader>) -> impl 'a + Iterator<Item=(usize, Vec<TTextureReader>)>
where
    TTextureReader: 'a + TextureReader<TTexture>,
    TTextureReader: Pixel<N>,
    TTexture:       Send + Sync,
{
    // Read about the texture
    let (width, height) = TTextureReader::texture_size(texture);
    let width_pixels    = width as usize;
    let height_pixels   = height as usize;

    // The read_row function reads a single row from the texture to be filtered
    let read_row = move |row_num| {
        TTextureReader::read_pixels_linear(texture, 0.0, 1.0, (1.0, 0.0), (0.0, row_num as f64), width_pixels)
    };

    apply_pixel_filter(width_pixels, read_row, height_pixels, filter)
}

///
/// Given a function that can read a row of pixels at a time, applies a texture filter and returns an iterator 
///
/// The 'read' function can assume that the only rows that will be read will be between 0..num_rows and each row will be requested at most once. Additionally, it may block
/// in order to ensure that the rows are read in order if needed.
///
pub fn apply_pixel_filter<'a, TPixel, const N: usize>(width: usize, read_row: impl 'a + Send + Sync + Fn(usize) -> Vec<TPixel>, num_rows: usize, filter: &'a impl PixelFilter<Pixel=TPixel>) -> impl 'a + Iterator<Item=(usize, Vec<TPixel>)>
where
    TPixel: 'a + Pixel<N>,
{
    use std::mem;

    // We need to add extra clear pixels above and below the 
    let (add_above, add_below) = filter.input_lines();
    let (add_left, add_right)  = filter.extra_columns();

    // The pixel cache stores the lines we've read from the source that we're going to supply to the filter. If we're rendering extra lines, 
    // then we allow each line to be read a certain number of times
    let pixel_cache = Arc::new(RwLock::new(HashMap::new()));

    let num_rows  = num_rows as isize;
    let add_above = add_above as isize;
    let add_below = add_below as isize;

    (0..num_rows)
        .map(move |row_num| {
            // Read the rows that the filter needs to operate
            let mut pixel_cache_read    = pixel_cache.read().unwrap();
            let mut pixel_rows          = Vec::with_capacity((add_above + add_below + 1) as usize);

            for read_row_num in (row_num-add_above)..=(row_num+add_below) {
                match pixel_cache_read.get(&read_row_num) {
                    None => {
                        // Read a row from the source
                        let mut new_row = if read_row_num < 0 || read_row_num >= num_rows {
                            vec![TPixel::default(); width]
                        } else {
                            read_row(read_row_num as usize)
                        };

                        // Add the 'add_left' and 'add_right' pixels
                        if add_left > 0 {
                            new_row.splice(0..0, (0..add_left).map(|_| TPixel::default()));
                        }

                        if add_right > 0 {
                            new_row.extend((0..add_right).map(|_| TPixel::default()));
                        }

                        // Keep the row with a reference
                        let new_row = Arc::new(new_row);

                        if add_above > 0 || add_below > 0 {
                            // Write the row to the pixel cache, along with a counter
                            // TODO: counter needs to account for the first and last set of rows
                            mem::drop(pixel_cache_read);
                            let mut pixel_cache_write = pixel_cache.write().unwrap();

                            pixel_cache_write.insert(read_row_num, (Arc::clone(&new_row), AtomicIsize::new(add_above + add_below)));

                            pixel_cache_read = pixel_cache.read().unwrap();
                        }

                        // Add to the list of rows that we're going to process
                        pixel_rows.push(new_row);
                    }

                    Some((pixel_row, usage_count)) => {
                        // Add the row to the result
                        pixel_rows.push(Arc::clone(pixel_row));

                        // Decrease the usage count
                        if usage_count.fetch_sub(1, Ordering::Acquire) <= 1 {
                            // Free the cached row as we've read it as many times as it's going to be used
                            mem::drop(pixel_cache_read);
                            let mut pixel_cache_write = pixel_cache.write().unwrap();

                            pixel_cache_write.remove(&read_row_num);

                            pixel_cache_read = pixel_cache.read().unwrap();
                        }
                    }
                }
            }

            // Filter the pixels
            let input_lines     = pixel_rows.iter().map(|pixels| &***pixels).collect::<Box<[_]>>();
            let mut output_line = vec![TPixel::default(); width];
            filter.filter_line(&*input_lines, &mut output_line);

            (row_num as usize, output_line)
        })
}

