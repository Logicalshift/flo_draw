use flo_render_software::pixel::*;

#[inline]
fn rgba_u16_rgba_round_trip() {
    // 256 * 256 image with all the alpha values
    let pixels = (0..=255)
        .flat_map(|y_pos| (0..=255).flat_map(move |x_pos| [x_pos, x_pos, x_pos, y_pos]))
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