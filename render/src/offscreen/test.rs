#[cfg(all(test, any(feature = "opengl")))]
mod test {
    use crate::action::*;
    use crate::buffer::*;
    use crate::offscreen::*;

    #[test]
    fn simple_offscreen_render() {
        // Initialise offscreen rendering
        let mut context     = initialize_offscreen_rendering().unwrap();

        // Draw a triangle in a 100x100 buffer
        use self::RenderAction::*;

        let mut renderer    = context.create_render_target(100, 100);
        let black           = [0, 0, 0, 255];
        renderer.render(vec![
            Clear(Rgba8([128, 128, 128, 255])),
            UseShader(ShaderType::Simple { erase_texture: None }),
            CreateVertex2DBuffer(VertexBufferId(0), vec![
                Vertex2D { pos: [-1.0, -1.0],   tex_coord: [0.0, 0.0], color: black },
                Vertex2D { pos: [1.0, 1.0],     tex_coord: [0.0, 0.0], color: black },
                Vertex2D { pos: [1.0, -1.0],    tex_coord: [0.0, 0.0], color: black },
            ]),
            DrawTriangles(VertexBufferId(0), 0..3)
        ]);

        let image           = renderer.realize();

        assert!(image.len() == 100*100*4);

        // First pixel should be black
        assert!(image[0] == 0);
        assert!(image[1] == 0);
        assert!(image[2] == 0);
        assert!(image[3] == 255);

        for y in 0..100 {
            for x in 0..100 {
                let pos         = (x + y*100) * 4;
                let pixel       = (image[pos], image[pos+1], image[pos+2], image[pos+3]);

                let expected    = if x >= y {
                    (0, 0, 0, 255)
                } else {
                    (128, 128, 128, 255)
                };

                if pixel != expected {
                    println!("{} {} {:?} {:?}", x, y, pixel, expected);
                }

                assert!(pixel == expected);
            }
        }
    }
}
