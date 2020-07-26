use super::stroke_settings::*;

use flo_render as render;

use lyon::tessellation::{VertexBuffers};

///
/// Operation to use when drawing an item on a layer
///
#[derive(Clone, Copy)]
pub enum LayerOperation {
    /// Draw the vertex buffer
    Draw,

    /// Erase the vertex buffer
    Erase
}

///
/// Single rendering operation for a layer
///
pub enum RenderEntity {
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
pub struct Layer {
    /// The render order for this layer
    pub render_order: Vec<RenderEntity>,

    /// The current fill colour
    pub fill_color: render::Rgba8,

    /// The settings for the next brush stroke
    pub stroke_settings: StrokeSettings
}
