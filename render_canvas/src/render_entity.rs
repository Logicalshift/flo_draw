use flo_canvas as canvas;
use flo_render as render;

use lyon::tessellation::{VertexBuffers};

///
/// How a vertex buffer is intended to be used
///
pub enum VertexBufferIntent {
    /// Will be drawn using DrawIndexed
    Draw,

    /// Will be rendered to the clipping area using EnableClipping
    Clip
}

///
/// Single rendering operation for a layer
///
pub enum RenderEntity {
    /// Render operation is missing
    Missing,

    /// Render operation is waiting to be tessellated (with a unique entity ID)
    Tessellating(usize),

    /// Tessellation waiting to be sent to the renderer
    VertexBuffer(VertexBuffers<render::Vertex2D, u16>, VertexBufferIntent),

    /// Render a vertex buffer
    DrawIndexed(render::VertexBufferId, render::IndexBufferId, usize),

    /// Render the sprite layer with the specified ID
    RenderSprite(canvas::SpriteId, canvas::Transform2D),

    /// Updates the transformation matrix for the layer
    SetTransform(canvas::Transform2D),

    /// Sets the blend mode to use for the following rendering
    SetBlendMode(render::BlendMode),

    /// Sets the dash pattern to use for the following rendering
    SetDashPattern(Vec<f32>),

    /// Sets the fill texture to use for the following rendering
    SetFillTexture(render::TextureId, render::Matrix, bool),

    /// Use the specified vertex buffer to define a clipping mask
    EnableClipping(render::VertexBufferId, render::IndexBufferId, usize),

    /// Stop clipping
    DisableClipping
}
