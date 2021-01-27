use flo_draw::*;
use flo_render::*;
use flo_stream::*;

use futures::prelude::*;
use futures::executor;

///
/// Simple example that displays a render window and does nothing with it
///
pub fn main() {
    // 'with_2d_graphics' is used to support operating systems that can't run event loops anywhere other than the main thread
    with_2d_graphics(|| {
        // Create a render window and loop until it stops sending events
        executor::block_on(async {
            // Create a window
            let (mut renderer, mut events) = create_render_window();

            // Render a triangle to it
            let black = [0, 0, 0, 255];
            renderer.publish(vec![
                RenderAction::Clear(Rgba8([128, 128, 128, 255])),
                RenderAction::SetTransform(Matrix::identity()),
                RenderAction::UseShader(ShaderType::Simple { erase_texture: None }),
                RenderAction::CreateVertex2DBuffer(VertexBufferId(0), vec![
                    Vertex2D { pos: [-1.0, -1.0],   tex_coord: [0.0, 0.0], color: black },
                    Vertex2D { pos: [1.0, 1.0],     tex_coord: [0.0, 0.0], color: black },
                    Vertex2D { pos: [1.0, -1.0],    tex_coord: [0.0, 0.0], color: black },
                ]),
                RenderAction::DrawTriangles(VertexBufferId(0), 0..3),

                RenderAction::ShowFrameBuffer
            ]).await;

            // Wait until it stops producing events
            while let Some(_evt) = events.next().await {
            }
        });
    });
}
