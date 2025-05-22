use flo_render_software::pixel::*;

#[test]
fn rgba_u16_rgba_round_trip_gamma10() {
    // 256 * 256 image with all the alpha values
    let pixels = (0..=255)
        .flat_map(|y_pos| (0..=255).flat_map(move |x_pos| [x_pos, y_pos, x_pos, 255]))
        .collect::<Vec<u8>>();

    let initial_texture = RgbaTexture::from_pixels(256, 256, pixels);
    let u16_texture     = U16LinearTexture::from_rgba(&initial_texture, 1.0);
    let final_texture   = RgbaTexture::from_linear_texture(&u16_texture, 1.0);

    for y_pos in 0..256 {
        let initial_pixels  = initial_texture.read_pixels((0..256).map(|x_pos| (x_pos, y_pos))).flatten().copied().collect::<Vec<u8>>();
        let final_pixels    = final_texture.read_pixels((0..256).map(|x_pos| (x_pos, y_pos))).flatten().copied().collect::<Vec<u8>>();

        assert!(initial_pixels == final_pixels, "{:?} != {:?}", initial_pixels, final_pixels);
    }
}

/* -- TODO: we currently don't have the precision to preserve the colours
#[test]
fn rgba_u16_rgba_round_trip_gamma_22() {
    // 256 * 256 image with all the alpha values
    let pixels = (0..=255)
        .flat_map(|y_pos| (0..=255).flat_map(move |x_pos| [x_pos, y_pos, x_pos, 255]))
        .collect::<Vec<u8>>();

    let initial_texture = RgbaTexture::from_pixels(256, 256, pixels);
    let u16_texture     = U16LinearTexture::from_rgba(&initial_texture, 2.2);
    let final_texture   = RgbaTexture::from_linear_texture(&u16_texture, 2.2);

    for y_pos in 0..256 {
        let initial_pixels  = initial_texture.read_pixels((0..256).map(|x_pos| (x_pos, y_pos))).flatten().copied().collect::<Vec<u8>>();
        let final_pixels    = final_texture.read_pixels((0..256).map(|x_pos| (x_pos, y_pos))).flatten().copied().collect::<Vec<u8>>();

        assert!(initial_pixels == final_pixels, "{:?} != {:?}", initial_pixels, final_pixels);
    }
}
*/

#[test]
fn generate_mipmap_level() {
    // 256 * 256 image with all the alpha values
    let pixels = (0..=255)
        .flat_map(|y_pos| (0..=255).flat_map(move |x_pos| [x_pos*257, x_pos*257, x_pos*257, y_pos*257]))
        .collect::<Vec<u16>>();

    // Generate a mip-map texture from the initial one
    let initial_texture = U16LinearTexture::from_pixels(256, 256, pixels);
    let mip_map_texture = initial_texture.create_mipmap().unwrap();

    assert!(mip_map_texture.width() == 128);
    assert!(mip_map_texture.height() == 128);

    for y_pos in 0..128 {
        // Read the pixels from the mip-map
        let mip_map_pixels = mip_map_texture.read_pixels((0..128).map(|x_pos| (x_pos, y_pos))).copied().collect::<Vec<_>>();

        for x_pos in 0..128 {
            let original_x = x_pos*2;
            let original_y = y_pos*2;
            let expected_r = (original_x*257 + (original_x+1)*257) / 2;
            let expected_a = (original_y*257 + (original_y+1)*257) / 2;

            let [r, _g, _b, a] = mip_map_pixels[x_pos];

            assert!(a == expected_a as u16, "({}, {}) a=={:?} (expected {:?})", x_pos, y_pos, a, expected_a);
            assert!(r == expected_r as u16, "({}, {}) r=={:?} (expected {:?})", x_pos, y_pos, r, expected_r);
        }
    }
}
