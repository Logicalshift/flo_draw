/// An identifier corresponding to a vertex buffer
#[derive(Clone, Copy, PartialEq, Debug, Hash)]
pub struct VertexBufferId(pub usize);

/// An identifier corresponding to an index buffer
#[derive(Clone, Copy, PartialEq, Debug, Hash)]
pub struct IndexBufferId(pub usize);

/// An identifier corresponding to a render target
#[derive(Clone, Copy, PartialEq, Debug, Hash)]
pub struct RenderTargetId(pub usize);

/// An identifier corresponding to a texture
#[derive(Clone, Copy, PartialEq, Debug, Hash)]
pub struct TextureId(pub usize);
