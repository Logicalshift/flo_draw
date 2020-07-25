use super::tessellate::*;

use flo_render as render;
use flo_canvas as canvas;
use flo_stream::*;

use ::desync::*;

use futures::prelude::*;
use num_cpus;
use lyon::tessellation::{VertexBuffers};

use std::sync::*;

///
/// Operation to use when drawing an item on a layer
///
#[derive(Clone, Copy)]
enum LayerOperation {
    /// Draw the vertex buffer
    Draw,

    /// Erase the vertex buffer
    Erase
}

///
/// Single rendering operation for a layer
///
enum RenderEntity {
    /// Render operation is waiting to be tessellated
    Tessellating(LayerOperation),

    /// Tessellation waiting to be sent to the renderer
    VertexBuffer(LayerOperation, VertexBuffers<render::Vertex2D, u16>),

    /// Render a vertex buffer
    DrawIndexed(LayerOperation, render::VertexBufferId, render::VertexBufferId)
}

///
/// Definition of a layer in the canvas
///
struct Layer {
    /// The render order for this layer
    render_order: Vec<RenderEntity>
}

///
/// Changes commands for `flo_canvas` into commands for `flo_render`
///
pub struct CanvasRenderer {
    /// The worker threads
    workers: Vec<Arc<Desync<Tessellator>>>,

    /// Layers defined by the canvas
    layers: Vec<Layer>
}

impl CanvasRenderer {
    ///
    /// Creates a new canvas renderer
    ///
    pub fn new() -> CanvasRenderer {
        // Create one worker per cpu
        let num_workers = num_cpus::get();
        let mut workers = Vec::with_capacity(num_workers);

        for _ in 0..num_workers {
            workers.push(Arc::new(Desync::new(Tessellator::new())));
        }

        // Generate the final renderer
        CanvasRenderer {
            workers:    workers,
            layers:     vec![]
        }
    }
}