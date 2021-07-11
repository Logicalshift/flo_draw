///
/// The input locations for the Metal vertex shaders
///
typedef enum VertexInputIndex {
    /// The transformation matrix
    VertexInputIndexMatrix      = 0,

    /// The vertices to render
    VertexInputIndexVertices    = 1,

    /// The texture transformation matrix
    VertexTextureMatrix         = 2
} VertexInputIndex;

///
/// The input locations for the Metal fragment shaders
///
typedef enum FragmentInputIndex {
    /// The texture to render
    FragmentIndexTexture            = 0,

    /// The eraser texture to render
    FragmentIndexEraseTexture       = 1,

    /// The clip mask texture to apply to the rendering
    FragmentIndexClipMaskTexture    = 2
} FragmentInputIndex;
