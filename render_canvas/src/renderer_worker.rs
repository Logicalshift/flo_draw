use super::renderer_core::*;
use super::renderer_layer::*;

use flo_render as render;
use ::desync::*;

use lyon::path;
use lyon::math::{Point};
use lyon::tessellation;
use lyon::tessellation::{VertexBuffers, BuffersBuilder, FillOptions, FillAttributes};

use std::sync::*;

///
/// References an entity in a layer
///
#[derive(Clone, Copy)]
pub struct LayerEntityRef {
    layer_id:       usize,
    entity_index:   usize
}

///
/// Describes a job for a canvas worker
///
pub enum CanvasJob {
    ///
    /// Tessellates a path by filling it
    ///
    Fill { 
        operation:      LayerOperation,
        path:           path::Path, 
        color:          render::Rgba8,
        entity:         LayerEntityRef
    }
}

///
/// State of a canvas worker
///
pub struct CanvasWorker {
    /// The core, where this worker will write its results
    core: Arc<Desync<RenderCore>>
}

impl CanvasWorker {
    ///
    /// Creates a new canvas worker
    ///
    pub fn new(core: &Arc<Desync<RenderCore>>) -> CanvasWorker {
        CanvasWorker {
            core: Arc::clone(core)
        }
    }

    ///
    /// Processes a single tessellation job (returning a vertex buffer entity)
    ///
    pub fn process_job(&mut self, job: CanvasJob) -> (LayerEntityRef, RenderEntity) {
        use self::CanvasJob::*;

        match job {
            Fill { operation, path, color, entity } => self.fill(operation, path, color, entity)
        }
    }

    ///
    /// Fills the current path and returns the resulting render entity
    ///
    fn fill(&mut self, operation: LayerOperation, path: path::Path, render::Rgba8(color): render::Rgba8, entity: LayerEntityRef) -> (LayerEntityRef, RenderEntity) {
        // Create the tessellator
        let mut tessellator = tessellation::FillTessellator::new();
        let mut geometry    = VertexBuffers::new();

        // Tessellate the current path
        tessellator.tessellate_path(&path, &FillOptions::default(),
            &mut BuffersBuilder::new(&mut geometry, move |point: Point, _attr: FillAttributes| {
                render::Vertex2D {
                    pos:        point.to_array(),
                    tex_coord:  [0.0, 0.0],
                    color:      color
                }
            })).unwrap();

        // Result is a vertex buffer render entity
        (entity, RenderEntity::VertexBuffer(operation, geometry))
    }
}
