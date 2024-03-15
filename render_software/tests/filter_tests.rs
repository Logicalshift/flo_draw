use flo_render_software::pixel::*;
use flo_render_software::filters::*;

use std::collections::{HashSet};

#[test]
pub fn alpha_blend_filter() {
    // We can apply the filter to some pixels we generate on the fly
    let filter          = AlphaBlendFilter::<F32LinearPixel, 4>::with_alpha(0.5);
    let alpha_blended   = apply_pixel_filter(256, |y_pos| {
        (0..256).map(|x_pos| F32LinearPixel::from_components([0.0, 0.0, (y_pos as f32)/256.0, (x_pos as f32)/256.0])).collect()
    }, 256, &filter);

    // Check that we receive each line once and they contain the expected set of pixels
    let mut visited_lines = HashSet::new();

    for (y_pos, pixels) in alpha_blended {
        assert!(y_pos < 256);

        // Don't allow duplicate lines
        assert!(!visited_lines.contains(&y_pos));
        visited_lines.insert(y_pos);

        // Check the pixels that are generated against their expected values
        for (x_pos, pixel) in pixels.into_iter().enumerate() {
            assert!(pixel == F32LinearPixel::from_components([0.0, 0.0, (y_pos as f32)/256.0/2.0, (x_pos as f32)/256.0/2.0]));
        }
    }

    assert!(visited_lines.len() == 256);
}

#[test]
pub fn smoke_test_horizontal_blur() {
    // We can apply the filter to some pixels we generate on the fly
    let filter          = HorizontalKernelFilter::<F32LinearPixel, 4>::with_gaussian_blur_radius(20.0);
    let alpha_blended   = apply_pixel_filter(256, |y_pos| {
        (0..256).map(|x_pos| F32LinearPixel::from_components([0.0, 0.0, (y_pos as f32)/256.0, (x_pos as f32)/256.0])).collect()
    }, 256, &filter);

    let _ = alpha_blended.collect::<Vec<_>>();
}

#[test]
pub fn smoke_test_vertical_blur() {
    // We can apply the filter to some pixels we generate on the fly
    let filter          = HorizontalKernelFilter::<F32LinearPixel, 4>::with_gaussian_blur_radius(20.0);
    let alpha_blended   = apply_pixel_filter(256, |y_pos| {
        (0..256).map(|x_pos| F32LinearPixel::from_components([0.0, 0.0, (y_pos as f32)/256.0, (x_pos as f32)/256.0])).collect()
    }, 256, &filter);

    let _ = alpha_blended.collect::<Vec<_>>();
}
