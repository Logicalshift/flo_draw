use std::f32;

///
/// Filters that can be applied to a texture by the rendering engine
///
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum TextureFilter {
    /// Applies a horizontal gaussian blur with the specified sigma (standard deviation) value, using a 9-pixel kernel
    GaussianBlurHorizontal9(f32),

    /// Applies a horizontal gaussian blur with the specified sigma (standard deviation) value, using a 29-pixel kernel
    GaussianBlurHorizontal29(f32),

    /// Applies a horizontal gaussian blur with the specified sigma (standard deviation) value, using a 61-pixel kernel
    GaussianBlurHorizontal61(f32),

    /// Applies a vertical gaussian blur with the specified sigma (standard deviation) value, using a 9-pixel kernel
    GaussianBlurVertical9(f32),

    /// Applies a vertical gaussian blur with the specified sigma (standard deviation) value, using a 9-pixel kernel
    GaussianBlurVertical29(f32),

    /// Applies a vertical gaussian blur with the specified sigma (standard deviation) value, using a 9-pixel kernel
    GaussianBlurVertical61(f32),

    /// Applies a gaussian blur in the horizontal direction with the specified sigma and kernel size
    GaussianBlurHorizontal(f32, usize),

    /// Applies a gaussian blur in the vertical direction with the specified sigma and kernel size
    GaussianBlurVertical(f32, usize),
}

impl TextureFilter {
    ///
    /// If this filter uses a kernel, returns the size to generate for the shader.
    ///
    /// This is the value to pass in for `weights_for_gaussian_blur`, so it's the half the total size of the kernel,
    /// plus 1 for the central value.
    ///
    pub (crate) fn kernel_size(&self) -> usize {
        use TextureFilter::*;

        match self {
            GaussianBlurHorizontal9(_)      => 5,
            GaussianBlurHorizontal29(_)     => 15,
            GaussianBlurHorizontal61(_)     => 31,
            GaussianBlurVertical9(_)        => 5,
            GaussianBlurVertical29(_)       => 15,
            GaussianBlurVertical61(_)       => 31,
            GaussianBlurHorizontal(_, size) => (size-1)/2+1,
            GaussianBlurVertical(_, size)   => (size-1)/2+1,
        }
    }

    ///
    /// Computes the 1D weights for a gaussian blur for a particular standard deviation
    ///
    pub (crate) fn weights_for_gaussian_blur(sigma: f32, count: usize) -> Vec<f32> {
        let sigma_squared = sigma * sigma;

        (0..count).into_iter()
            .map(|x| {
                let x = x as f32;
                (1.0/((2.0*f32::consts::PI*sigma_squared).sqrt())) * (f32::consts::E.powf(-(x*x)/(2.0*sigma_squared)))
            })
            .collect()
    }

    ///
    /// Transforms the weights for the gaussian blur to a set of offsets and weights that can be used
    /// with bilinear texture filtering
    ///
    /// See See <https://www.rastergrid.com/blog/2010/09/efficient-gaussian-blur-with-linear-sampling/> for a
    /// description of this algorithm
    ///
    pub (crate) fn weights_and_offsets_for_gaussian_blur(weights: Vec<f32>) -> (Vec<f32>, Vec<f32>) {
        let mut new_weights = vec![weights[0]];
        let mut new_offsets = vec![0.0];

        let mut idx = 1;
        while idx < weights.len()-1 {
            let offset1 = idx as f32;
            let offset2 = (idx+1) as f32;

            let new_weight = weights[idx] + weights[idx+1];
            new_weights.push(new_weight);
            new_offsets.push((offset1*weights[idx] + offset2*weights[idx+1])/new_weight);

            idx += 2;
        }

        (new_weights, new_offsets)
    }
}
