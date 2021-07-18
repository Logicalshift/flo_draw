use flo_draw::*;
use flo_draw::canvas::*;
use flo_draw::render_canvas::*;
use ::desync::*;

use futures::prelude::*;
use futures::stream;
use futures::executor;

use std::mem;
use std::sync::*;
use std::collections::{HashMap};

///
/// Draws a circle, then intercepts the tessellation and displays that instead
///
/// flo_draw sends graphics to and from its various subsystems using streams rather than
/// method calls: this makes it very easy to intercept and change what's being sent around.
/// In this case we take the instructions just before they would have been sent to the GPU
/// and display them as line art, showing what the GPU would be rendering.
///
/// These actions are performed by the flo_render_canvas library. The instructions aren't 
/// specific to any particular API at this point, so they can be used to implement rendering 
/// on APIs or libraries that aren't explicitly supported by `flo_render` or to interoperate
/// with other rendering systems such as that which might be present in a game engine or a
/// UI layer. Note that there's no need to set up callbacks or implement traits in order to
/// retrieve this part of the state of the renderer.
///
/// It's possible to create a window that renders the GPU instructions directly by calling
/// `create_render_window`.
///
pub fn main() {
    with_2d_graphics(|| {
        // Create a drawing target. canvas is our renderer, and stream is where these instructions are sent to
        let (canvas, canvas_stream) = DrawingTarget::new();

        // Create a canvas renderer, and wrap it in a Desync. 
        // The canvas renderer generates instructions for a GPU-based renderer.
        // Desync is a companion library; it provides a convenient API for many asynchronous tasks, in this case stream processing
        let renderer                = CanvasRenderer::new();
        let renderer                = Arc::new(Desync::new(renderer));

        // Configure it to a viewport of 768x768 (want 'square pixels' so the rendering isn't squashed later on)
        renderer.desync(|renderer| { 
            renderer.set_viewport(0.0..768.0, 0.0..768.0, 768.0, 768.0, 1.0);
        });

        // Use Desync to process the rendering instructions on the stream (returning a stream of vecs, which we flatten to a stream of single instructions)
        let canvas_stream           = canvas_stream.ready_chunks(1000);
        let mut gpu_instructions    = pipe(renderer, canvas_stream, |renderer, drawing_instructions| {
            async move {
                renderer.draw(drawing_instructions.into_iter())
                    .collect::<Vec<_>>()
                    .await
            }.boxed()
        }).map(|as_vectors| stream::iter(as_vectors)).flatten();

        // Draw a circle that will get sent to the renderer we just set up (rather than directly to the window)
        canvas.draw(|gc| {
            // Set up the canvas
            gc.canvas_height(1000.0);
            gc.center_region(0.0, 0.0, 1000.0, 1000.0);

            // Draw a circle
            gc.new_path();
            gc.circle(500.0, 500.0, 250.0);

            gc.fill_color(Color::Rgba(0.3, 0.6, 0.8, 1.0));
            gc.fill();

            gc.line_width(6.0);
            gc.stroke_color(Color::Rgba(0.0, 0.0, 0.0, 1.0));
            gc.stroke();
        });

        // Dropping the canvas closes the stream so the list of drawing instructions ends
        mem::drop(canvas);

        // Create a window to render on
        let tessellation_window     = create_drawing_window("Circle Tessellation");

        // Run an executor to track the instructions that we would be sending to the GPU and render them to the tessellation window instead
        executor::block_on(async {
            // We to keep the vertex buffers around so we can render them once we get the index buffers
            let mut vertex_buffers  = HashMap::new();
            let mut index_buffers   = HashMap::new();

            while let Some(gpu_instruction) = gpu_instructions.next().await {
                // Render the tessellation to the tesselator window
                match &gpu_instruction {
                    RenderAction::SetTransform(Matrix(t)) => {
                        // Set an approximate equivalent of the transform the 'draw' instruction generated
                        tessellation_window.draw(|gc| {
                            gc.canvas_height(2.0);
                            gc.center_region(0.0, 0.0, 2.0, 2.0);
                            gc.transform(Transform2D([
                                [t[0][0], t[0][1], t[0][2]],
                                [t[1][0], t[1][1], t[1][2]],
                                [t[2][0], t[2][1], t[2][2]]
                            ]));
                        })
                    }

                    RenderAction::CreateVertex2DBuffer(buffer_id, vertices) => {
                        // Store the vertex buffer: we can render it when we get the corresponding index buffer
                        vertex_buffers.insert(*buffer_id, vertices.clone());
                    }

                    RenderAction::CreateIndexBuffer(buffer_id, indicies) => { 
                        // Store the index buffer for when we receive the rendering instruction
                        index_buffers.insert(*buffer_id, indicies.clone());
                    }

                    RenderAction::DrawIndexedTriangles(vertex_buffer_id, index_buffer_id, num_vertices) => {
                        // Fetch the buffers
                        let vertices = vertex_buffers.get(&vertex_buffer_id).unwrap();
                        let indicies = index_buffers.get(&index_buffer_id).unwrap();

                        tessellation_window.draw(|gc| {
                            // Render triangles from the index buffer
                            for triangle_num in 0..(num_vertices/3) {
                                // Use the index buffer to look up the vertices for this triangle
                                let index           = triangle_num * 3;
                                let i1: u16         = indicies[index+0];
                                let i2: u16         = indicies[index+1];
                                let i3: u16         = indicies[index+2];
                                let p1: &Vertex2D   = &vertices[i1 as usize];
                                let p2: &Vertex2D   = &vertices[i2 as usize];
                                let p3: &Vertex2D   = &vertices[i3 as usize];

                                let colour          = Color::Rgba(p1.color[0] as f32/255.0, p1.color[1] as f32/255.0, p1.color[2] as f32/255.0, p1.color[3] as f32/255.0);

                                // Render as lines
                                gc.new_path();

                                gc.move_to(p1.pos[0], p1.pos[1]);
                                gc.line_to(p2.pos[0], p2.pos[1]);
                                gc.line_to(p3.pos[0], p3.pos[1]);
                                gc.close_path();

                                gc.stroke_color(colour);
                                gc.stroke();
                            }
                        });
                    }

                    _ => {}
                }
            }
        })
    });
}
