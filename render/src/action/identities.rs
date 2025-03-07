/// An identifier corresponding to a vertex buffer
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct VertexBufferId(pub usize);

/// An identifier corresponding to an index buffer
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct IndexBufferId(pub usize);

/// An identifier corresponding to a render target
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct RenderTargetId(pub usize);

/// An identifier corresponding to a texture
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct TextureId(pub usize);
