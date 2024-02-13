use super::pixel_trait::*;

///
/// Trait implemented by types that can be read from a texture type
///
pub trait TextureReader<TTexture, const N: usize> : Pixel<N> + Send + Sync + Sized
where
    TTexture: Send + Sync,
{
    ///
    /// Reads a sequence of pixels from this texture into a target array
    ///
    /// Coordinates are in fractions of pixels to allow for a texture reader to support bilinear interpolation or mipmapping
    ///
    fn read_pixels(texture: &TTexture, positions: &[(f64, f64)]) -> Vec<Self>;

    ///
    /// Reads a set of pixels across a linear gradient
    ///
    /// This is a common way that pixels are read out from a texture, so this function can be overridden to optimise this
    /// for different types of texture storage if needed.
    ///
    /// This reads `count` pixels at locations `t = 0, 1, 2, ...` such that `u = dx * t + offset` and `x = x_gradient.0 * u + x_gradient.1`,
    /// `y = y_gradient.0 * u + y_gradient.1`.
    ///
    #[inline]
    fn read_pixels_linear(texture: &TTexture, offset:f64, dx: f64, x_gradient: (f64, f64), y_gradient: (f64, f64), count: usize) -> Vec<Self> {
        // Allocate enough space to store the pixels
        let mut positions = Vec::with_capacity(count);

        // Calculate the positions for the pixels
        positions.extend((0..count).map(|t| {
            let t = t as f64;
            let u = dx * t + offset;
            let x = x_gradient.0 * u + x_gradient.1;
            let y = y_gradient.0 * u + y_gradient.1;

            (x, y)
        }));

        Self::read_pixels(texture, &positions)
    }

    ///
    /// Reads pixels and applies bilinear filtering to approximate values found at subpixels
    ///
    /// This can be used for scaling up an image or scaling down an image to about half size
    ///
    fn read_pixels_bilinear_filter(texture: &TTexture, positions: &[(f64, f64)]) -> Vec<Self> {
        // Create the resulting pixels for this read
        let mut result          = Vec::with_capacity(positions.len());

        // If there's nothing to read, then short-circuit
        if positions.is_empty() {
            return result;
        }

        // In order to minimize the amount of reading we do, we make a plan of the pixels we're going to read (for each pixel in the output we need a 2x2 sample set)
        enum Action {
            NextQuad,
            ReadPixel(f64, f64),
        }
        let mut actions         = Vec::with_capacity(positions.len()*2);
        let mut pixels_to_read  = Vec::with_capacity(positions.len()*4);

        // We always start by reading the first position (the 4 pixels surrounding xpos, ypos)
        let (mut xpos, mut ypos) = positions[0];

        xpos = xpos.floor();
        ypos = ypos.floor();

        pixels_to_read.extend([
            (xpos, ypos), (xpos+1.0, ypos), (xpos, ypos+1.0), (xpos+1.0, ypos+1.0)
        ]);

        // Generate the actions and the pixels to read
        for (next_x, next_y) in positions {
            // Read another set of pixels if the current pixel doesn't match
            if xpos != next_x.floor() || ypos != next_y.floor() {
                xpos = next_x.floor();
                ypos = next_y.floor();

                pixels_to_read.extend([
                    (xpos, ypos), (xpos+1.0, ypos), (xpos, ypos+1.0), (xpos+1.0, ypos+1.0)
                ]);
                actions.push(Action::NextQuad);
            }

            // Interpolate the next pixel
            actions.push(Action::ReadPixel(next_x - xpos, next_y - ypos));
        }

        // Read the data we need from the texture and then perform the actions
        let source_pixels       = Self::read_pixels(texture, &pixels_to_read);
        let mut pos             = 0;
        let mut current_pixels  = [&source_pixels[0], &source_pixels[1], &source_pixels[2], &source_pixels[3]];

        for action in actions {
            match action {
                Action::NextQuad => {
                    pos             += 4;
                    current_pixels  = [&source_pixels[pos+0], &source_pixels[pos+1], &source_pixels[pos+2], &source_pixels[pos+3]];
                },

                Action::ReadPixel(offset_x, offset_y) => {
                    result.push(Self::filter_bilinear(current_pixels, offset_x, offset_y));
                }
            }
        }

        result
    }
}
