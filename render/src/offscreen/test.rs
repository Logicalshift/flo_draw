#[cfg(all(test, any(feature = "opengl", feature = "osx-metal")))]
mod test {
    use crate::action::*;
    use crate::buffer::*;
    use crate::offscreen::*;

    #[test]
    fn clear_offscreen() {
        // Initialise offscreen rendering
        let context         = initialize_offscreen_rendering();
        let mut context     = match context {
            Ok(context)     => context,
            Err(RenderInitError::CannotCreateGraphicsDevice)    => { println!("Test not run: graphics device unavailable"); return; }
            Err(other)      => { panic!("Unexpected error: {:?}", other); }
        };

        // Draw a triangle in a 100x100 buffer
        use self::RenderAction::*;

        let mut renderer    = context.create_render_target(100, 100);
        renderer.render(vec![
            Clear(Rgba8([128, 128, 128, 255])),
        ]);

        let image           = renderer.realize();

        assert!(image.len() == 100*100*4);

        assert!(image[0] == 128);
        assert!(image[1] == 128);
        assert!(image[2] == 128);
        assert!(image[3] == 255);

        for y in 0..100 {
            for x in 0..100 {
                let pos         = (x + y*100) * 4;
                let pixel       = (image[pos], image[pos+1], image[pos+2], image[pos+3]);

                let expected    = (128, 128, 128, 255);

                if pixel != expected {
                    println!("{} {} {:?} {:?}", x, y, pixel, expected);
                }

                assert!(pixel == expected);
            }
        }
    }

    #[test]
    fn clears_in_rgba_order() {
        // Initialise offscreen rendering
        let context         = initialize_offscreen_rendering();
        let mut context     = match context {
            Ok(context)     => context,
            Err(RenderInitError::CannotCreateGraphicsDevice)    => { println!("Test not run: graphics device unavailable"); return; }
            Err(other)      => { panic!("Unexpected error: {:?}", other); }
        };

        // Draw a triangle in a 100x100 buffer
        use self::RenderAction::*;

        let mut renderer    = context.create_render_target(100, 100);
        renderer.render(vec![
            Clear(Rgba8([128, 129, 130, 255])),
        ]);

        let image           = renderer.realize();

        assert!(image.len() == 100*100*4);

        assert!(image[0] == 128);
        assert!(image[1] == 129);
        assert!(image[2] == 130);
        assert!(image[3] == 255);

        for y in 0..100 {
            for x in 0..100 {
                let pos         = (x + y*100) * 4;
                let pixel       = (image[pos], image[pos+1], image[pos+2], image[pos+3]);

                let expected    = (128, 129, 130, 255);

                if pixel != expected {
                    println!("{} {} {:?} {:?}", x, y, pixel, expected);
                }

                assert!(pixel == expected);
            }
        }
    }

    #[test]
    fn simple_offscreen_render() {
        // Initialise offscreen rendering
        let context         = initialize_offscreen_rendering();
        let mut context     = match context {
            Ok(context)     => context,
            Err(RenderInitError::CannotCreateGraphicsDevice)    => { println!("Test not run: graphics device unavailable"); return; }
            Err(other)      => { panic!("Unexpected error: {:?}", other); }
        };

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

    #[test]
    fn simple_offscreen_render_with_transform() {
        // Initialise offscreen rendering
        let context         = initialize_offscreen_rendering();
        let mut context     = match context {
            Ok(context)     => context,
            Err(RenderInitError::CannotCreateGraphicsDevice)    => { println!("Test not run: graphics device unavailable"); return; }
            Err(other)      => { panic!("Unexpected error: {:?}", other); }
        };

        // Draw a triangle in a 100x100 buffer
        use self::RenderAction::*;

        let mut renderer    = context.create_render_target(100, 100);
        let black           = [0, 0, 0, 255];
        renderer.render(vec![
            Clear(Rgba8([128, 128, 128, 255])),
            SetTransform(Matrix::identity()),
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

    #[test]
    fn offscreen_order_is_rgba() {
        // Initialise offscreen rendering
        let context         = initialize_offscreen_rendering();
        let mut context     = match context {
            Ok(context)     => context,
            Err(RenderInitError::CannotCreateGraphicsDevice)    => { println!("Test not run: graphics device unavailable"); return; }
            Err(other)      => { panic!("Unexpected error: {:?}", other); }
        };

        // Draw a triangle in a 100x100 buffer
        use self::RenderAction::*;

        let mut renderer    = context.create_render_target(100, 100);
        let black           = [1, 2, 3, 255];
        renderer.render(vec![
            Clear(Rgba8([128, 129, 130, 255])),
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

        println!("({:x}, {:x}, {:x}, {:x})", image[0], image[1], image[2], image[3]);

        assert!(image[0] == 1);
        assert!(image[1] == 2);
        assert!(image[2] == 3);
        assert!(image[3] == 255);

        for y in 0..100 {
            for x in 0..100 {
                let pos         = (x + y*100) * 4;
                let pixel       = (image[pos], image[pos+1], image[pos+2], image[pos+3]);

                let expected    = if x >= y {
                    (1, 2, 3, 255)
                } else {
                    (128, 129, 130, 255)
                };

                if pixel != expected {
                    println!("{} {} {:?} {:?}", x, y, pixel, expected);
                }

                assert!(pixel == expected);
            }
        }
    }
}
