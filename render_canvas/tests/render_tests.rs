use flo_render::*;
use flo_render as render;
use flo_render_canvas::*;
use flo_canvas::*;

use futures::prelude::*;
use futures::executor;

///
/// Checks that the instructions beginning a new layer are valid
///
async fn check_layer_preamble<S: Unpin+Stream<Item=RenderAction>>(stream: &mut S) {
    let select_render_target = stream.next().await;
    println!("{:?}", select_render_target);
    assert!(match select_render_target { Some(RenderAction::SelectRenderTarget(_)) => true, _ => false });

    let set_blend_mode = stream.next().await;
    println!("{:?}", set_blend_mode);
    assert!(match set_blend_mode { Some(RenderAction::BlendMode(render::BlendMode::SourceOver)) => true, _ => false });

    let use_shader = stream.next().await;
    println!("{:?}", use_shader);
    assert!(match use_shader { Some(RenderAction::UseShader(_)) => true, _ => false });

    let set_transform = stream.next().await;
    println!("{:?}", set_transform);
    assert!(match set_transform { Some(RenderAction::SetTransform(_)) => true, _ => false });
}

#[test]
fn fill_simple_circle() {
    // Draw a simple circle
    let mut draw_circle = vec![];
    draw_circle.circle(0.0,0.0, 100.0);
    draw_circle.fill();

    executor::block_on(async {
        // Create the renderer
        let mut renderer    = CanvasRenderer::new();

        // Get the upates for a drawing operation
        let mut draw_stream = renderer.draw(draw_circle.into_iter());

        // Rendering starts at a 'clear', after some pre-rendering instructions, an 'upload vertex buffer', an 'upload index buffer' and a 'draw indexed'
        loop {
            let next = draw_stream.next().await;
            assert!(next.is_some());

            if let Some(RenderAction::Clear(_)) = &next {
                break;
            }
        }

        let set_transform   = draw_stream.next().await;
        println!("{:?}", set_transform);
        assert!(set_transform.is_some());
        assert!(match set_transform { Some(RenderAction::SetTransform(_)) => true, _ => false });

        let upload_vertices = draw_stream.next().await;
        println!("{:?}", upload_vertices);
        assert!(upload_vertices.is_some());
        assert!(match upload_vertices { Some(RenderAction::CreateVertex2DBuffer(_, _)) => true, _ => false });

        let upload_indices  = draw_stream.next().await;
        println!("{:?}", upload_indices);
        assert!(upload_indices.is_some());
        assert!(match upload_indices { Some(RenderAction::CreateIndexBuffer(_, _)) => true, _ => false });

        // Layer preamble occurs after uploading the buffers
        check_layer_preamble(&mut draw_stream).await;

        let draw_vertices   = draw_stream.next().await;
        println!("{:?}", draw_vertices);
        assert!(draw_vertices.is_some());
        assert!(match draw_vertices { Some(RenderAction::DrawIndexedTriangles(_, _, _)) => true, _ => false });

        // Stream then has some post-rendering instructions
    })
}

#[test]
fn fill_two_circles() {
    // Draw a simple circle
    let mut draw_circle = vec![];
    draw_circle.circle(0.0,0.0, 100.0);
    draw_circle.fill();
    draw_circle.fill();

    executor::block_on(async {
        // Create the renderer
        let mut renderer    = CanvasRenderer::new();

        // Get the upates for a drawing operation
        let mut draw_stream = renderer.draw(draw_circle.into_iter());

        // Should be a 'clear', an 'upload vertex buffer', an 'upload index buffer' and two 'draw indexed' instructions
        loop {
            let next = draw_stream.next().await;
            assert!(next.is_some());

            if let Some(RenderAction::Clear(_)) = &next {
                break;
            }
        }

        let set_transform   = draw_stream.next().await;
        assert!(set_transform.is_some());
        assert!(match set_transform { Some(RenderAction::SetTransform(_)) => true, _ => false });

        // First we upload the vertex buffers...
        let upload_vertices = draw_stream.next().await;
        assert!(upload_vertices.is_some());
        assert!(match upload_vertices { Some(RenderAction::CreateVertex2DBuffer(_, _)) => true, _ => false });

        let upload_indices  = draw_stream.next().await;
        assert!(upload_indices.is_some());
        assert!(match upload_indices { Some(RenderAction::CreateIndexBuffer(_, _)) => true, _ => false });

        let upload_vertices_2 = draw_stream.next().await;
        assert!(upload_vertices_2.is_some());
        assert!(match upload_vertices_2 { Some(RenderAction::CreateVertex2DBuffer(_, _)) => true, _ => false });

        let upload_indices_2 = draw_stream.next().await;
        assert!(upload_indices_2.is_some());
        assert!(match upload_indices_2 { Some(RenderAction::CreateIndexBuffer(_, _)) => true, _ => false });

        // Layer preamble occurs after uploading the buffers
        check_layer_preamble(&mut draw_stream).await;

        // Drawing starts after the layer preamble
        let draw_vertices   = draw_stream.next().await;
        assert!(draw_vertices.is_some());
        assert!(match draw_vertices { Some(RenderAction::DrawIndexedTriangles(_, _, _)) => true, _ => false });

        let draw_vertices_2  = draw_stream.next().await;
        assert!(draw_vertices_2.is_some());
        assert!(match draw_vertices_2 { Some(RenderAction::DrawIndexedTriangles(_, _, _)) => true, _ => false });
    })
}

#[test]
fn draw_twice() {
    // Draw a simple circle
    let mut draw_circle = vec![];
    draw_circle.circle(0.0,0.0, 100.0);
    draw_circle.fill();

    executor::block_on(async {
        // Create the renderer
        let mut renderer        = CanvasRenderer::new();

        {
            // Get the upates for a drawing operation
            let mut draw_stream     = renderer.draw(draw_circle.into_iter());

            // Should be a 'clear', an 'upload vertex buffer', an 'upload index buffer' and a 'draw indexed'
            loop {
                let next = draw_stream.next().await;
                assert!(next.is_some());

                if let Some(RenderAction::Clear(_)) = &next {
                    break;
                }
            }

            let _set_transform      = draw_stream.next().await;
            let _upload_vertices    = draw_stream.next().await;
            let _upload_indices     = draw_stream.next().await;
            let _draw_vertices      = draw_stream.next().await;
        }

        // Draw again: re-render without regenerating the buffers
        let mut draw_stream = renderer.draw(vec![].into_iter());

        // Should be a 'clear', and a 'draw indexed'
        loop {
            let next = draw_stream.next().await;
            assert!(next.is_some());

            if let Some(RenderAction::Clear(_)) = &next {
                break;
            }
        }

        let set_transform   = draw_stream.next().await;
        assert!(set_transform.is_some());
        assert!(match set_transform { Some(RenderAction::SetTransform(_)) => true, _ => false });

        check_layer_preamble(&mut draw_stream).await;

        let draw_vertices   = draw_stream.next().await;
        assert!(draw_vertices.is_some());
        assert!(match draw_vertices { Some(RenderAction::DrawIndexedTriangles(_, _, _)) => true, _ => false });
    })
}

#[test]
fn clip_rect() {
    // Draw a simple rectabgle
    let mut clip_rect = vec![];
    clip_rect.new_path();
    clip_rect.rect(0.0,0.0, 100.0, 100.0);
    clip_rect.clip();

    executor::block_on(async {
        // Create the renderer
        let mut renderer    = CanvasRenderer::new();

        // Get the upates for a drawing operation
        let mut draw_stream = renderer.draw(clip_rect.into_iter());

        // Rendering starts at a 'clear', after some pre-rendering instructions, an 'upload vertex buffer', an 'upload index buffer' and a 'draw indexed'
        loop {
            let next = draw_stream.next().await;
            assert!(next.is_some());

            if let Some(RenderAction::Clear(_)) = &next {
                break;
            }
        }

        // Read the next few instructions
        let mut rendering = vec![];
        for _ in 0..19 {
            rendering.push(draw_stream.next().await.unwrap());
        }

        println!("{:?}", rendering);

        // Should start by initialising the vertex buffers
        use self::RenderAction::*;
        assert!(match rendering[0] { SetTransform(_) => true, _ => false });
        assert!(match rendering[1] { CreateVertex2DBuffer(_, _) => true, _ => false });
        assert!(match rendering[2] { CreateIndexBuffer(_, _) => true, _ => false });
        assert!(match rendering[3] { SelectRenderTarget(RenderTargetId(0)) => true, _ => false });

        // Then set up to render to the clip texture (render target 2)
        assert!(match rendering[4] { BlendMode(render::BlendMode::AllChannelAlphaSourceOver) => true, _ => false });
        assert!(match rendering[5] { UseShader(render::ShaderType::Simple { clip_texture: None, erase_texture: None }) => true, _ => false });
        assert!(match rendering[6] { SetTransform(_) => true, _ => false });
        assert!(match rendering[7] { SelectRenderTarget(RenderTargetId(1)) => true, _ => false });
        assert!(match rendering[8] { Clear(Rgba8([0,0,0,255])) => true, _ => false });

        // Render the clipping texture
        assert!(match rendering[9] { DrawIndexedTriangles(_, _, _) => true, _ => false });

        // Finally, resets the state for rendering to the main view with a clipping region (texture ID 2 has the clip region in it)
        assert!(match rendering[10] { SelectRenderTarget(RenderTargetId(0)) => true, _ => false });
        assert!(match rendering[11] { BlendMode(render::BlendMode::SourceOver) => true, _ => false });
        assert!(match rendering[12] { UseShader(render::ShaderType::Simple { clip_texture: Some(render::TextureId(1)), erase_texture: None }) => true, _ => false });
        assert!(match rendering[13] { SetTransform(_) => true, _ => false });

        // Remaining instructions finish the render
    })
}
