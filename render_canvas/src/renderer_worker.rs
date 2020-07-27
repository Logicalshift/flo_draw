use super::renderer_layer::*;

use flo_render as render;

use lyon::path;
use lyon::math::{Point};
use lyon::tessellation;
use lyon::tessellation::{VertexBuffers, BuffersBuilder, FillOptions, FillRule, FillAttributes};

///
/// References an entity in a layer
///
#[derive(Clone, Copy)]
pub struct LayerEntityRef {
    pub layer_id:           usize,
    pub entity_index:       usize,
    pub layer_generation:   usize
}

///
/// Describes a job for a canvas worker
///
pub enum CanvasJob {
    ///
    /// Tessellates a path by filling it
    ///
    Fill { 
        operation:  LayerOperation,
        path:       path::Path, 
        color:      render::Rgba8,
        entity:     LayerEntityRef
    }
}

///
/// State of a canvas worker
///
pub struct CanvasWorker {
}

impl CanvasWorker {
    ///
    /// Creates a new canvas worker
    ///
    pub fn new() -> CanvasWorker {
        CanvasWorker {
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
        let mut tessellator     = tessellation::FillTessellator::new();
        let mut geometry        = VertexBuffers::new();

        let mut fill_options    = FillOptions::default();
        fill_options.fill_rule  = FillRule::NonZero;

        // Tessellate the current path
        tessellator.tessellate_path(&path, &fill_options,
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
