use flo_render::*;
use flo_render_canvas::*;
use flo_canvas::*;

use futures::prelude::*;
use futures::executor;

///
/// Checks that the instructions beginning a new layer are valid
///
async fn check_layer_preamble<S: Unpin+Stream<Item=RenderAction>>(stream: &mut S) {
    let select_render_target = stream.next().await;
    assert!(match select_render_target { Some(RenderAction::SelectRenderTarget(_)) => true, _ => false });

    let use_shader = stream.next().await;
    assert!(match use_shader { Some(RenderAction::UseShader(_)) => true, _ => false });

    let set_transform = stream.next().await;
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
        assert!(set_transform.is_some());
        assert!(match set_transform { Some(RenderAction::SetTransform(_)) => true, _ => false });

        let upload_vertices = draw_stream.next().await;
        assert!(upload_vertices.is_some());
        assert!(match upload_vertices { Some(RenderAction::CreateVertex2DBuffer(_, _)) => true, _ => false });

        let upload_indices  = draw_stream.next().await;
        assert!(upload_indices.is_some());
        assert!(match upload_indices { Some(RenderAction::CreateIndexBuffer(_, _)) => true, _ => false });

        // Layer preamble occurs after uploading the buffers
        check_layer_preamble(&mut draw_stream).await;

        let draw_vertices   = draw_stream.next().await;
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
