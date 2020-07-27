use flo_render::*;
use flo_render_canvas::*;
use flo_canvas::*;

use futures::prelude::*;
use futures::executor;

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

        // Should be a 'clear', an 'upload vertex buffer', an 'upload index buffer' and a 'draw indexed'
        let clear           = draw_stream.next().await;
        assert!(clear.is_some());
        assert!(match clear { Some(RenderAction::Clear(_)) => true, _ => false });

        let upload_vertices = draw_stream.next().await;
        assert!(upload_vertices.is_some());
        assert!(match upload_vertices { Some(RenderAction::CreateVertex2DBuffer(_, _)) => true, _ => false });

        let upload_indices  = draw_stream.next().await;
        assert!(upload_indices.is_some());
        assert!(match upload_indices { Some(RenderAction::CreateIndexBuffer(_, _)) => true, _ => false });

        let draw_vertices   = draw_stream.next().await;
        assert!(draw_vertices.is_some());
        assert!(match draw_vertices { Some(RenderAction::DrawIndexedTriangles(_, _, _)) => true, _ => false });

        let end_of_stream   = draw_stream.next().await;
        assert!(end_of_stream.is_none());
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
        let clear           = draw_stream.next().await;
        assert!(clear.is_some());
        assert!(match clear { Some(RenderAction::Clear(_)) => true, _ => false });

        let upload_vertices = draw_stream.next().await;
        assert!(upload_vertices.is_some());
        assert!(match upload_vertices { Some(RenderAction::CreateVertex2DBuffer(_, _)) => true, _ => false });

        let upload_indices  = draw_stream.next().await;
        assert!(upload_indices.is_some());
        assert!(match upload_indices { Some(RenderAction::CreateIndexBuffer(_, _)) => true, _ => false });

        let draw_vertices   = draw_stream.next().await;
        assert!(draw_vertices.is_some());
        assert!(match draw_vertices { Some(RenderAction::DrawIndexedTriangles(_, _, _)) => true, _ => false });

        let upload_vertices_2 = draw_stream.next().await;
        assert!(upload_vertices_2.is_some());
        assert!(match upload_vertices_2 { Some(RenderAction::CreateVertex2DBuffer(_, _)) => true, _ => false });

        let upload_indices_2 = draw_stream.next().await;
        assert!(upload_indices_2.is_some());
        assert!(match upload_indices_2 { Some(RenderAction::CreateIndexBuffer(_, _)) => true, _ => false });

        let draw_vertices_2  = draw_stream.next().await;
        assert!(draw_vertices_2.is_some());
        assert!(match draw_vertices_2 { Some(RenderAction::DrawIndexedTriangles(_, _, _)) => true, _ => false });

        let end_of_stream   = draw_stream.next().await;
        assert!(end_of_stream.is_none());
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
            let _clear              = draw_stream.next().await;
            let _upload_vertices    = draw_stream.next().await;
            let _upload_indices     = draw_stream.next().await;
            let _draw_vertices      = draw_stream.next().await;
            let end_of_stream       = draw_stream.next().await;
            assert!(end_of_stream.is_none());
        }

        // Draw again: re-render without regenerating the buffers
        let mut draw_stream = renderer.draw(vec![].into_iter());

        // Should be a 'clear', an 'upload vertex buffer', an 'upload index buffer' and a 'draw indexed'
        let clear           = draw_stream.next().await;
        assert!(clear.is_some());
        assert!(match clear { Some(RenderAction::Clear(_)) => true, _ => false });

        let draw_vertices   = draw_stream.next().await;
        assert!(draw_vertices.is_some());
        assert!(match draw_vertices { Some(RenderAction::DrawIndexedTriangles(_, _, _)) => true, _ => false });

        let end_of_stream   = draw_stream.next().await;
        assert!(end_of_stream.is_none());
    })
}
