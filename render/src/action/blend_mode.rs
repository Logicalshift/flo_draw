///
/// The blending modes that the renderer must support (most of the Porter-Duff modes)
///
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
pub enum BlendMode {
    SourceOver,
    DestinationOver,
    SourceIn,
    DestinationIn,
    SourceOut,
    DestinationOut,
    SourceATop,
    DestinationATop,

    Screen,
    Multiply,

    AllChannelAlphaSourceOver,
    AllChannelAlphaDestinationOver
}
