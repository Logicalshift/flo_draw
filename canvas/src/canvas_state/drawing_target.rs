use crate::draw::*;

///
/// Where the next item to be drawn should go
///
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum DrawingTarget {
    Layer(LayerId),
    Sprite(SpriteId),
}
