use flo_render_software::pixel::*;

use flo_canvas::*;

#[test]
fn from_color() {
    let linear_pixel    = F32LinearPixel::from_color(Color::Rgba(0.1, 0.2, 0.3, 1.0), 2.2);
    let back_as_color   = linear_pixel.to_color(2.2);

    let (r, g, b, a)    = back_as_color.to_rgba_components();

    assert!((r-0.1).abs() < 0.001, "red={:?}", r);
    assert!((g-0.2).abs() < 0.001, "green={:?}", g);
    assert!((b-0.3).abs() < 0.001, "blue={:?}", b);
    assert!((a-1.0).abs() < 0.001, "alpha={:?}", a);
}

#[test]
fn gamma_correction_1() {
    let linear_pixel    = F32LinearPixel::from_color(Color::Rgba(0.1, 0.2, 0.3, 1.0), 2.2);
    let back_as_color   = linear_pixel.to_color(1.0);

    let (r, g, b, a)    = back_as_color.to_rgba_components();

    assert!((r-0.1f32.powf(2.2)).abs() < 0.001, "red={:?}", r);
    assert!((g-0.2f32.powf(2.2)).abs() < 0.001, "green={:?}", g);
    assert!((b-0.3f32.powf(2.2)).abs() < 0.001, "blue={:?}", b);
    assert!((a-1.0).abs() < 0.001, "alpha={:?}", a);
}

#[test]
fn gamma_correction_2() {
    let linear_pixel    = F32LinearPixel::from_color(Color::Rgba(0.1, 0.2, 0.3, 1.0), 1.0);
    let back_as_color   = linear_pixel.to_color(2.2);

    let (r, g, b, a)    = back_as_color.to_rgba_components();

    assert!((r-0.1f32.powf(0.4545)).abs() < 0.001, "red={:?}", r);
    assert!((g-0.2f32.powf(0.4545)).abs() < 0.001, "green={:?}", g);
    assert!((b-0.3f32.powf(0.4545)).abs() < 0.001, "blue={:?}", b);
    assert!((a-1.0).abs() < 0.001, "alpha={:?}", a);
}

#[test]
fn gamma_correction_3() {
    let linear_pixel    = F32LinearPixel::from_color(Color::Rgba(1.0, 1.0, 1.0, 0.5), 2.2);
    let back_as_color   = linear_pixel.to_color(1.0);

    let (r, g, b, a)    = back_as_color.to_rgba_components();

    assert!((r-1.0).abs() < 0.001, "red={:?}", r);
    assert!((g-1.0).abs() < 0.001, "green={:?}", g);
    assert!((b-1.0).abs() < 0.001, "blue={:?}", b);
    assert!((a-0.5).abs() < 0.001, "alpha={:?}", a);
}

#[test]
fn gamma_correction_4() {
    let linear_pixel    = F32LinearPixel::from_color(Color::Rgba(0.0, 0.0, 0.0, 0.25), 2.2);
    let back_as_color   = linear_pixel.to_color(1.0);

    let (r, g, b, a)    = back_as_color.to_rgba_components();

    assert!((r-0.0).abs() < 0.001, "red={:?}", r);
    assert!((g-0.0).abs() < 0.001, "green={:?}", g);
    assert!((b-0.0).abs() < 0.001, "blue={:?}", b);
    assert!((a-0.25).abs() < 0.001, "alpha={:?}", a);
}

#[test]
fn source_over_1() {
    let col1 = F32LinearPixel::from_color(Color::Rgba(0.4980, 0.6039, 0.7647, 1.0), 2.2);
    let col2 = F32LinearPixel::from_color(Color::Rgba(0.7764, 0.6823, 0.8588, 0.6), 2.2);

    let col3            = col2.source_over(col1);
    let back_as_color   = col3.to_color(2.2);

    let (r, g, b, a)    = back_as_color.to_rgba_components();

    debug_assert!((r-0.6666).abs() < 0.025, "r({}, {}, {}, {})", r, g, b, a);
    debug_assert!((g-0.6509).abs() < 0.025, "g({}, {}, {}, {})", r, g, b, a);
    debug_assert!((b-0.8196).abs() < 0.025, "b({}, {}, {}, {})", r, g, b, a);
    debug_assert!((a-1.0).abs() < 0.01, "({}, {}, {}, {})", r, g, b, a);
}

#[test]
fn source_over_2() {
    let col1 = F32LinearPixel::from_color(Color::Rgba(0.4980, 0.6039, 0.7647, 1.0), 2.2);
    let col2 = F32LinearPixel::from_color(Color::Rgba(0.7764, 0.6823, 0.8588, 0.6), 2.2);

    let col3                = col2.source_over(col1);
    let col3                = [col3];
    let mut back_as_color   = [U8RgbaPremultipliedPixel::default()];
    F32LinearPixel::to_gamma_colorspace(&col3, &mut back_as_color, 2.2);
    let back_as_color       = back_as_color[0];

    let [r, g, b, a]    = back_as_color.get_components();

    debug_assert!(r == 173, "r({}, {}, {}, {})", r, g, b, a);
    debug_assert!(g == 166, "g({}, {}, {}, {})", r, g, b, a);
    debug_assert!(b == 209, "b({}, {}, {}, {})", r, g, b, a);
    debug_assert!(a == 255, "a({}, {}, {}, {})", r, g, b, a);
}


#[test]
fn source_over_3() {
    let col1        = F32LinearPixel::from_color(Color::Rgba(0.4980, 0.6039, 0.7647, 1.0), 2.2);
    let col2        = F32LinearPixel::from_color(Color::Rgba(0.7764, 0.6823, 0.8588, 0.6), 2.2);
    let src_over    = AlphaOperation::SourceOver.get_function();

    let col3                = src_over(col2, col1);
    let col3                = [col3];
    let mut back_as_color   = [U8RgbaPremultipliedPixel::default()];
    F32LinearPixel::to_gamma_colorspace(&col3, &mut back_as_color, 2.2);
    let back_as_color       = back_as_color[0];

    let [r, g, b, a]    = back_as_color.get_components();

    debug_assert!(r == 173, "r({}, {}, {}, {})", r, g, b, a);
    debug_assert!(g == 166, "g({}, {}, {}, {})", r, g, b, a);
    debug_assert!(b == 209, "b({}, {}, {}, {})", r, g, b, a);
    debug_assert!(a == 255, "a({}, {}, {}, {})", r, g, b, a);
}

#[test]
fn bilinear_interpolate_1() {
    let col1 = F32LinearPixel::from_color(Color::Rgba(0.0, 1.0, 0.25, 1.0), 1.0);
    let col2 = F32LinearPixel::from_color(Color::Rgba(1.0, 0.0, 0.75, 1.0), 1.0);
    let col3 = F32LinearPixel::from_color(Color::Rgba(0.0, 1.0, 0.25, 1.0), 1.0);
    let col4 = F32LinearPixel::from_color(Color::Rgba(1.0, 0.0, 0.75, 1.0), 1.0);

    let interpolate_mid = F32LinearPixel::filter_bilinear([&col1, &col2, &col3, &col4], 0.5, 0.5);

    let [r, g, b, a] = interpolate_mid.to_components();

    assert!((r-0.5).abs() < 0.001, "{:?}", [r, g, b, a]);
    assert!((g-0.5).abs() < 0.001, "{:?}", [r, g, b, a]);
    assert!((b-0.5).abs() < 0.001, "{:?}", [r, g, b, a]);
    assert!((a-1.0).abs() < 0.001, "{:?}", [r, g, b, a]);
}

#[test]
fn bilinear_interpolate_2() {
    let col1 = F32LinearPixel::from_color(Color::Rgba(0.0, 1.0, 0.25, 1.0), 1.0);
    let col2 = F32LinearPixel::from_color(Color::Rgba(1.0, 0.0, 0.75, 1.0), 1.0);
    let col3 = F32LinearPixel::from_color(Color::Rgba(0.0, 1.0, 0.25, 1.0), 1.0);
    let col4 = F32LinearPixel::from_color(Color::Rgba(1.0, 0.0, 0.75, 1.0), 1.0);

    let interpolate_mid = F32LinearPixel::filter_bilinear([&col1, &col2, &col3, &col4], 0.0, 0.0);

    let [r, g, b, a] = interpolate_mid.to_components();

    assert!((r-0.0).abs() < 0.001, "{:?}", [r, g, b, a]);
    assert!((g-1.0).abs() < 0.001, "{:?}", [r, g, b, a]);
    assert!((b-0.25).abs() < 0.001, "{:?}", [r, g, b, a]);
    assert!((a-1.0).abs() < 0.001, "{:?}", [r, g, b, a]);
}

#[test]
fn bilinear_interpolate_3() {
    let col1 = F32LinearPixel::from_color(Color::Rgba(0.0, 1.0, 0.25, 1.0), 1.0);
    let col2 = F32LinearPixel::from_color(Color::Rgba(1.0, 0.0, 0.75, 1.0), 1.0);
    let col3 = F32LinearPixel::from_color(Color::Rgba(0.0, 1.0, 0.25, 1.0), 1.0);
    let col4 = F32LinearPixel::from_color(Color::Rgba(1.0, 0.0, 0.75, 1.0), 1.0);

    let interpolate_mid = F32LinearPixel::filter_bilinear([&col1, &col2, &col3, &col4], 1.0, 1.0);

    let [r, g, b, a] = interpolate_mid.to_components();

    assert!((r-1.0).abs() < 0.001, "{:?}", [r, g, b, a]);
    assert!((g-0.0).abs() < 0.001, "{:?}", [r, g, b, a]);
    assert!((b-0.75).abs() < 0.001, "{:?}", [r, g, b, a]);
    assert!((a-1.0).abs() < 0.001, "{:?}", [r, g, b, a]);
}

#[test]
fn bilinear_interpolate_4() {
    let col1 = F32LinearPixel::from_color(Color::Rgba(0.0, 1.0, 0.25, 1.0), 1.0);
    let col2 = F32LinearPixel::from_color(Color::Rgba(1.0, 0.0, 0.75, 1.0), 1.0);
    let col3 = F32LinearPixel::from_color(Color::Rgba(0.0, 1.0, 0.25, 1.0), 1.0);
    let col4 = F32LinearPixel::from_color(Color::Rgba(1.0, 0.0, 0.75, 1.0), 1.0);

    let interpolate_mid = F32LinearPixel::filter_bilinear([&col1, &col2, &col3, &col4], 0.25, 0.25);

    let [r, g, b, a] = interpolate_mid.to_components();

    assert!((r-0.25).abs() < 0.001, "{:?}", [r, g, b, a]);
    assert!((g-0.75).abs() < 0.001, "{:?}", [r, g, b, a]);
    assert!((b-0.375).abs() < 0.001, "{:?}", [r, g, b, a]);
    assert!((a-1.0).abs() < 0.001, "{:?}", [r, g, b, a]);
}
