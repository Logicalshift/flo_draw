use flo_canvas as canvas;
use flo_render as render;

use lyon::tessellation::{VertexBuffers};

///
/// Single rendering operation for a layer
///
pub enum RenderEntity {
    /// Render operation is missing
    Missing,

    /// Render operation is waiting to be tessellated (with a unique entity ID)
    Tessellating(usize),

    /// Tessellation waiting to be sent to the renderer
    VertexBuffer(VertexBuffers<render::Vertex2D, u16>),

    /// Render a vertex buffer
    DrawIndexed(render::VertexBufferId, render::IndexBufferId, usize),

    /// Render the sprite layer with the specified ID
    RenderSprite(canvas::SpriteId, canvas::Transform2D),

    /// Updates the transformation matrix for the layer
    SetTransform(canvas::Transform2D),

    /// Sets the blend mode to use for the following rendering
    SetBlendMode(render::BlendMode),

    /// Clears the current render target
    Clear(render::Rgba8),

    /// Sets the current render target
    SetRenderTarget(render::RenderTargetId)
}
