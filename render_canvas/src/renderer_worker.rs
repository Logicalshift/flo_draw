use super::render_entity::*;
use super::renderer_core::*;
use super::stroke_settings::*;

use flo_render as render;
use flo_canvas as canvas;

use lyon::path;
use lyon::math::{Point};
use lyon::tessellation;
use lyon::tessellation::{VertexBuffers, BuffersBuilder, StrokeOptions, FillOptions, FillRule, FillAttributes, StrokeAttributes};

/// The minimum tolerance to use when rendering fills/strokes
const MIN_TOLERANCE: f32 = 0.0001;

/// The maximum tolerance to use when rendering fills/strokes
const MAX_TOLERANCE: f32 = 1000.0;

///
/// References an entity in a layer
///
#[derive(Clone, Copy)]
pub struct LayerEntityRef {
    pub layer_id:           LayerHandle,
    pub entity_index:       usize,
    pub entity_id:          usize
}

///
/// Describes a job for a canvas worker
///
pub enum CanvasJob {
    ///
    /// Tessellates a path by filling it
    ///
    Fill { 
        path:           path::Path, 
        color:          render::Rgba8,
        fill_rule:      FillRule,
        scale_factor:   f64,
        entity:         LayerEntityRef
    },

    Stroke {
        path:           path::Path,
        stroke_options: StrokeSettings,
        scale_factor:   f64,
        entity:         LayerEntityRef
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
            Fill    { path, fill_rule, color, scale_factor, entity }    => self.fill(path, fill_rule, color, scale_factor, entity),
            Stroke  { path, stroke_options, scale_factor, entity }      => self.stroke(path, stroke_options, scale_factor, entity)
        }
    }

    ///
    /// Fills the current path and returns the resulting render entity
    ///
    fn fill(&mut self, path: path::Path, fill_rule: FillRule, render::Rgba8(color): render::Rgba8, scale_factor: f64, entity: LayerEntityRef) -> (LayerEntityRef, RenderEntity) {
        // Create the tessellator and geometry
        let mut tessellator     = tessellation::FillTessellator::new();
        let mut geometry        = VertexBuffers::new();

        // Set up the fill options
        let mut fill_options    = FillOptions::default();
        fill_options.fill_rule  = fill_rule;
        fill_options.tolerance  = FillOptions::DEFAULT_TOLERANCE * (scale_factor as f32);
        fill_options.tolerance  = f32::min(MAX_TOLERANCE, fill_options.tolerance);
        fill_options.tolerance  = f32::max(MIN_TOLERANCE, fill_options.tolerance);

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
        (entity, RenderEntity::VertexBuffer(geometry))
    }

    ///
    /// Converts some stroke settings to Lyon stroke options
    ///
    fn convert_stroke_settings(stroke_settings: StrokeSettings) -> StrokeOptions {
        let mut stroke_options = StrokeOptions::default();

        stroke_options.line_width   = stroke_settings.line_width;
        stroke_options.end_cap      = match stroke_settings.cap {
            canvas::LineCap::Butt   => tessellation::LineCap::Butt,
            canvas::LineCap::Square => tessellation::LineCap::Square,
            canvas::LineCap::Round  => tessellation::LineCap::Round

        };
        stroke_options.start_cap    = stroke_options.end_cap;
        stroke_options.line_join    = match stroke_settings.join {
            canvas::LineJoin::Miter => tessellation::LineJoin::Miter,
            canvas::LineJoin::Bevel => tessellation::LineJoin::Bevel,
            canvas::LineJoin::Round => tessellation::LineJoin::Round
        };

        stroke_options
    }

    ///
    /// Strokes a path and returns the resulting render entity
    ///
    fn stroke(&mut self, path: path::Path, stroke_options: StrokeSettings, scale_factor: f64, entity: LayerEntityRef) -> (LayerEntityRef, RenderEntity) {
        // Create the tessellator and geometry
        let mut tessellator         = tessellation::StrokeTessellator::new();
        let mut geometry            = VertexBuffers::new();

        // Set up the stroke options
        let render::Rgba8(color)    = stroke_options.stroke_color;
        let mut stroke_options      = Self::convert_stroke_settings(stroke_options);
        stroke_options.tolerance    = StrokeOptions::DEFAULT_TOLERANCE * (scale_factor as f32);
        stroke_options.tolerance    = f32::min(MAX_TOLERANCE, stroke_options.tolerance);
        stroke_options.tolerance    = f32::max(MIN_TOLERANCE, stroke_options.tolerance);

        // Stroke the path
        tessellator.tessellate_path(&path, &stroke_options,
            &mut BuffersBuilder::new(&mut geometry, move |point: Point, _attr: StrokeAttributes| {
                render::Vertex2D {
                    pos:        point.to_array(),
                    tex_coord:  [0.0, 0.0],
                    color:      color
                }
            })).unwrap();

        // Result is a vertex buffer render entity
        (entity, RenderEntity::VertexBuffer(geometry))
    }
}
