use flo_draw::*;
use flo_render::*;
use flo_stream::*;
use flo_render as render;

use futures::prelude::*;
use futures::executor;

///
/// Does a double resolve of a multisampled render target (testing out which direction things end up in, as OpenGL might flip things around)
///
pub fn main() {
    // 'with_2d_graphics' is used to support operating systems that can't run event loops anywhere other than the main thread
    with_2d_graphics(|| {
        // Create a render window and loop until it stops sending events
        executor::block_on(async {
            use self::RenderAction::*;

            // Create a window
            let (mut renderer, mut events) = create_render_window("Direct render action window");

            let render_actions = vec![
                // Create a couple of multisampled targets
                RenderAction::CreateRenderTarget(RenderTargetId(0), TextureId(0), Size2D(768, 768), RenderTargetType::Multisampled),
                RenderAction::CreateRenderTarget(RenderTargetId(1), TextureId(1), Size2D(768, 768), RenderTargetType::Multisampled),

                RenderAction::SetTransform(Matrix([[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 1.0], [0.0, 0.0, 0.0, 1.0]])),
                RenderAction::BlendMode(render::BlendMode::SourceOver),
                RenderAction::UseShader(ShaderType::Simple { clip_texture: None }),

                // Render a triangle to render target 0
                SelectRenderTarget(RenderTargetId(0)),
                Clear(Rgba8([255, 220, 220, 255])),
                CreateVertex2DBuffer(VertexBufferId(1), vec![
                    Vertex2D::with_pos(-0.5, -0.5).with_color(1.0, 0.0, 0.0, 1.0),
                    Vertex2D::with_pos(-0.0, 0.5).with_color(1.0, 0.0, 0.0, 1.0),
                    Vertex2D::with_pos(0.5, -0.5).with_color(1.0, 0.0, 0.0, 1.0),
                ]),
                RenderAction::CreateIndexBuffer(IndexBufferId(1), vec![0, 1, 2]),
                DrawIndexedTriangles(VertexBufferId(1), IndexBufferId(1), 3),

                // Resolve render target 0 to 1
                SelectRenderTarget(RenderTargetId(1)),
                Clear(Rgba8([255, 255, 255, 255])),
                RenderAction::DrawFrameBuffer(RenderTargetId(0), FrameBufferRegion::default(), Alpha(1.0)),              

                // Resolve render target 1 to the framebuffer
                RenderToFrameBuffer,
                Clear(Rgba8([255, 255, 255, 255])),

                //RenderAction::DrawFrameBuffer(RenderTargetId(0), FrameBufferRegion::default(), Alpha(1.0)),              
                RenderAction::DrawFrameBuffer(RenderTargetId(1), FrameBufferRegion::default(), Alpha(1.0)),              

                ShowFrameBuffer,
            ];

            // Render the instructions generaated by the show_tessellation example
            renderer.publish(render_actions.clone()).await;

            // Wait until it stops producing events
            while let Some(evt) = events.next().await {
                // Stop reading events when the window is closed (this will close our streams, so the window will disappear)
                match evt {
                    DrawEvent::Redraw   => {
                        renderer.publish(render_actions.clone()).await;
                    }
                    DrawEvent::Closed   => { break; }
                    _                   => { }
                }
            }
        });
    });
}
