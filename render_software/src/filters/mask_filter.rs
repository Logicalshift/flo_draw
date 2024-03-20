use crate::pixel::*;

use std::sync::*;
use std::marker::{PhantomData};

///
/// The mask filter multiplies the output pixels by the contents of a mask texture
///
pub struct MaskFilter<TPixel, const N: usize>
where
    TPixel: Pixel<N>,
{
    mask:   Arc<U16LinearTexture>,
    mult_x: f64,
    mult_y: f64,
    pixel:  PhantomData<TPixel>,
}

impl<TPixel, const N: usize> MaskFilter<TPixel, N> 
where
    TPixel: Pixel<N>,
{
    ///
    /// Creates a new mask filter that will use the alpha value from the specified texture to mask the input texture
    ///
    pub fn with_mask(mask: &Arc<U16LinearTexture>, multiply_x: f64, multiply_y: f64) -> Self {
        MaskFilter {
            mask:   Arc::clone(mask),
            mult_x: multiply_x,
            mult_y: multiply_y,
            pixel:  PhantomData,
        }
    }

    ///
    /// Reads the red and green fraction of the pixels given the lower and upper lines, x position and y fraction
    ///
    #[inline]
    fn read_px(&self, xpos: usize, line_pixels_1: &[U16LinearPixel], line_pixels_2: &[U16LinearPixel], ypos_fract: u32) -> u16 {
        // Calculate the x position along the lines by multiplying by the map position
        let xpos        = xpos as f64 * self.mult_x;
        let xpos        = xpos.abs() % line_pixels_1.len() as f64;
        let xpos_fract  = xpos.fract();
        let xpos_fract  = (xpos_fract * 65535.0) as u32;
        let xpos        = xpos as usize;
        let xpos_1      = (xpos+1) % line_pixels_1.len();

        // Read the 4 corners of the pixel
        let px1 = line_pixels_1[xpos];
        let px2 = line_pixels_1[xpos_1];
        let px3 = line_pixels_2[xpos];
        let px4 = line_pixels_2[xpos];

        // Only need the alpha channel: calculate the value using bilinear filtering
        let a1 = px1.a() as u32;
        let a2 = px2.a() as u32;
        let a3 = px3.a() as u32;
        let a4 = px4.a() as u32;

        let a12 = ((a1 * xpos_fract)>>16) + ((a2 * (65535-xpos_fract))>>16);
        let a34 = ((a3 * xpos_fract)>>16) + ((a4 * (65535-xpos_fract))>>16);

        let a = ((a12 * ypos_fract)>>16) + ((a34 * (65535-ypos_fract))>>16);

        a as u16
    }
  }