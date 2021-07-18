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
/// Draws some text and then shows how it's tessellated
///
/// This is a more advanced version of show_tessellation that additionally demonstrates how to turn
/// font rendering instructions into normal vector instructions for post-processing (another example
/// that demonstrates this is 'Wibble')
///
pub fn main() {
    with_2d_graphics(|| {
        let lato        = CanvasFontFace::from_slice(include_bytes!("Lato-Regular.ttf"));

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

        // Render the text as vectors by processing the canvas stream
        let canvas_stream           = drawing_with_laid_out_text(canvas_stream);
        let canvas_stream           = drawing_with_text_as_paths(canvas_stream);

        // Use Desync to process the rendering instructions on the stream (returning a stream of vecs, which we flatten to a stream of single instructions)
        let canvas_stream           = canvas_stream.ready_chunks(1000);
        let mut gpu_instructions    = pipe(renderer, canvas_stream, |renderer, drawing_instructions| {
            async move {
                renderer.draw(drawing_instructions.into_iter())
                    .collect::<Vec<_>>()
                    .await
            }.boxed()
        }).map(|as_vectors| stream::iter(as_vectors)).flatten();

        // Say 'hello, world' on the canvas that we're processing
        let hello_size  = measure_text(&lato, "Hello, World", 100.0);
        let (min, max)  = hello_size.inner_bounds;

        let x_pos       = (1000.0 - (max.x()-min.x()))/2.0;
        let y_pos       = (1000.0 - (max.y()-min.y()))/2.0;

        canvas.draw(|gc| {
            // Set up the canvas
            gc.canvas_height(1000.0);
            gc.center_region(0.0, 0.0, 1000.0, 1000.0);

            // Load a font
            gc.define_font_data(FontId(1), Arc::clone(&lato));
            gc.set_font_size(FontId(1), 100.0);

            // Draw some text in our font
            gc.fill_color(Color::Rgba(0.0, 0.0, 0.6, 1.0));
            gc.draw_text(FontId(1), "Hello, World".to_string(), x_pos as _, y_pos as _);
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
