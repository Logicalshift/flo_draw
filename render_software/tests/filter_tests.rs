use flo_render_software::pixel::*;
use flo_render_software::filters::*;

#[test]
pub fn alpha_blend_filter() {
    // We can apply the filter to some pixels we generate on the fly
    let filter          = AlphaBlendFilter::<F32LinearPixel, 4>::with_alpha(0.5);
    let alpha_blended   = apply_pixel_filter(256, (0..256).map(|y_pos| {
        (0..256).map(|x_pos| F32LinearPixel::from_components([0.0, 0.0, (y_pos as f32)/256.0, (x_pos as f32)/256.0])).collect()
    }), &filter);

    let mut num_visited_lines = 0;
    for (y_pos, pixels) in alpha_blended.enumerate() {
        num_visited_lines += 1;

        // Check the pixels that are generated against their expected values
        for (x_pos, pixel) in pixels.into_iter().enumerate() {
            assert!(pixel == F32LinearPixel::from_components([0.0, 0.0, (y_pos as f32)/256.0/2.0, (x_pos as f32)/256.0/2.0]));
        }
    }

    assert!(num_visited_lines == 256);
}

#[test]
pub fn smoke_test_horizontal_blur() {
    // We can apply the filter to some pixels we generate on the fly
    let filter  = HorizontalKernelFilter::<F32LinearPixel, 4>::with_gaussian_blur_radius(20.0);
    let blurred = apply_pixel_filter(256, (0..256).map(|y_pos| {
        (0..256).map(|x_pos| F32LinearPixel::from_components([0.0, 0.0, (y_pos as f32)/256.0, (x_pos as f32)/256.0])).collect()
    }), &filter);

    let _ = blurred.collect::<Vec<_>>();
}

#[test]
pub fn smoke_test_vertical_blur() {
    // We can apply the filter to some pixels we generate on the fly
    let filter  = VerticalKernelFilter::<F32LinearPixel, 4>::with_gaussian_blur_radius(20.0);
    let blurred = apply_pixel_filter(256, (0..256).map(|y_pos| {
        (0..256).map(|x_pos| F32LinearPixel::from_components([0.0, 0.0, (y_pos as f32)/256.0, (x_pos as f32)/256.0])).collect()
    }), &filter);

    let _ = blurred.collect::<Vec<_>>();
}

#[test]
pub fn horizontal_blur_0() {
    // We can apply the filter to some pixels we generate on the fly
    let filter  = HorizontalKernelFilter::<F32LinearPixel, 4>::with_gaussian_blur_radius(0.0);
    let blurred = apply_pixel_filter(256, (0..256).map(|y_pos| {
        (0..256).map(|x_pos| F32LinearPixel::from_components([0.0, 0.0, (y_pos as f32)/256.0, (x_pos as f32)/256.0])).collect()
    }), &filter);

    for (y_pos, pixels) in blurred.enumerate() {
        assert!(y_pos < 256);

        // Check the pixels that are generated against their expected values
        for (x_pos, pixel) in pixels.into_iter().enumerate() {
            let expected = F32LinearPixel::from_components([0.0, 0.0, (y_pos as f32)/256.0, (x_pos as f32)/256.0]);
            assert!(pixel == expected, "{:?} != {:?} ({:?})", pixel, expected, pixel - expected);
        }
    }
}

#[test]
pub fn vertical_blur_0() {
    // We can apply the filter to some pixels we generate on the fly
    let filter  = VerticalKernelFilter::<F32LinearPixel, 4>::with_gaussian_blur_radius(0.0);
    let blurred = apply_pixel_filter(256, (0..256).map(|y_pos| {
        (0..256).map(|x_pos| F32LinearPixel::from_components([0.0, 0.0, (y_pos as f32)/256.0, (x_pos as f32)/256.0])).collect()
    }), &filter);

    for (y_pos, pixels) in blurred.enumerate() {
        assert!(y_pos < 256);

        // Check the pixels that are generated against their expected values
        for (x_pos, pixel) in pixels.into_iter().enumerate() {
            let expected = F32LinearPixel::from_components([0.0, 0.0, (y_pos as f32)/256.0, (x_pos as f32)/256.0]);
            assert!(pixel == expected, "{:?} != {:?} ({:?})", pixel, expected, pixel - expected);
        }
    }
}
