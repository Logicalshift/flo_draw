///
/// The input locations for the Metal vertex shaders
///
typedef enum VertexInputIndex {
    /// The transformation matrix
    VertexInputIndexMatrix      = 0,

    /// The vertices to render
    VertexInputIndexVertices    = 1
} VertexInputIndex;

///
/// The input locations for the Metal fragment shaders
///
typedef enum FragmentInputIndex {
    /// The texture to render
    FragmentIndexTexture        = 0
} FragmentInputIndex;
