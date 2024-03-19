use super::pixel_filter_trait::*;
use crate::pixel::*;

use std::sync::*;
use std::marker::{PhantomData};

///
/// A displacement map filter reads from a target texture and displaces each pixel by a specific amount 
///
pub struct DisplacementMapFilter<TPixel, const N: usize>
where
    TPixel: Pixel<N>,
{
    gamma_lookup:       Box<[u16; 65536]>,
    displacement_map:   Arc<U16LinearTexture>,
    offset_x:           f64,
    offset_y:           f64,
    map_mult_x:         f64,
    map_mult_y:         f64,
    pixel:              PhantomData<TPixel>,
}

impl<TPixel, const N: usize> DisplacementMapFilter<TPixel, N>
where
    TPixel: Pixel<N>,
{
    ///
    /// Creates a new displacement map filter
    ///
    /// The offsets here are the maximum value in pixels that the image can move away from its original value. The multiplication factors
    /// are used to translate coordinates from the displacement map to the 
    ///
    /// The gamma correction value is applied after reading from the map texture (so we can get linear distortions
    /// from a gamma-corrected texture)
    ///
    pub fn with_displacement_map(map: &Arc<U16LinearTexture>, offset_x: f64, offset_y: f64, multiply_x: f64, multiply_y: f64, gamma: f64) -> Self {
        let mut gamma_lookup = [0u16; 65536];

        for pos in 0..65536 {
            gamma_lookup[pos] = ((pos as f64/65535.0).powf(1.0/gamma) * 65535.0).round() as u16;
        }

        DisplacementMapFilter {
            gamma_lookup:       Box::new(gamma_lookup),
            displacement_map:   Arc::clone(map),
            offset_x:           offset_x,
            offset_y:           offset_y,
            map_mult_x:         multiply_x,
            map_mult_y:         multiply_y,
            pixel:              PhantomData,
        }
    }
}

impl<TPixel, const N: usize> PixelFilter for DisplacementMapFilter<TPixel, N>
where
    TPixel: Pixel<N>,
{
    type Pixel = TPixel;

    fn input_lines(&self) -> (usize, usize) {
        (self.offset_y.abs().ceil() as usize, self.offset_y.abs().ceil() as usize + 1)
    }

    fn extra_columns(&self) -> (usize, usize) {
        (self.offset_x.abs().ceil() as usize, self.offset_x.abs().ceil() as usize + 1)
    }

    fn filter_line(&self, ypos: usize, input_lines: &[&[Self::Pixel]], output_line: &mut [Self::Pixel]) {
        let mid_point_x = self.offset_x.abs().ceil();
        let mid_point_y = self.offset_y.abs().ceil();

        // Read a line from the displacement map
        let line_pixels = self.displacement_map.pixel_line(ypos);
        let num_extra   = (output_line.len() as isize - self.displacement_map.width() as isize).max(0);

        if let Some(line_pixels) = line_pixels {
            // Read from the input using the offsets from the displacement map
            let line_pixels = U16LinearPixel::u16_slice_as_linear_pixels_immutable(line_pixels);
            let gamma_lut   = &*self.gamma_lookup;

            for (output_x, px) in line_pixels.iter().copied().chain((0..num_extra).map(|_| U16LinearPixel::from_components([32767, 32767, 32767, 32767]))).enumerate().take(output_line.len()) {
                // Read the x and y offsets from the texture
                let x_off = ((gamma_lut[px.r() as usize] as f64)/65535.0) * self.offset_x * 2.0;
                let y_off = ((gamma_lut[px.g() as usize] as f64)/65535.0) * self.offset_y * 2.0;

                // The pixel we read is at a particular x, y position
                let xpos = output_x + x_off as usize;
                let ypos = y_off as usize;

                // Read the 4 pixels for bilinear filtering
                let pixels = [
                    &input_lines[ypos][xpos],
                    &input_lines[ypos][xpos+1],
                    &input_lines[ypos+1][xpos],
                    &input_lines[ypos+1][xpos+1],
                ];

                // Filter the result to generate the final pixel
                output_line[output_x] = TPixel::filter_bilinear(pixels, TPixel::Component::with_value(x_off.fract()), TPixel::Component::with_value(y_off.fract()));
            }
        } else {
            // Just copy the mid-point pixels to the output (outside of the displacement map)
            let len = output_line.len();
            output_line[0..len].copy_from_slice(&input_lines[mid_point_y as usize][(mid_point_x as usize)..(len + mid_point_x as usize)])
        }
    }
}