///
/// Identifier of a canvas 'sprite'
///
/// A 'sprite' is just a placeholder for a set of pre-rendered actions (it's useful for things like
/// images or drawings that are expected to repeat). Sprites survive layer and canvas clears so they
/// can be re-used repeatedly. The drawing layer may cache these actions in order to render the sprite
/// quickly.
///
/// Sprites are also faster to draw when rendering to a remote surface as they only need to be sent
/// across once before they can be re-rendered as often as necessary.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpriteId(pub u64);

///
/// A position within a sprite
///
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SpritePosition(pub f32, pub f32);

///
/// A size within a sprite
///
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SpriteSize(pub f32, pub f32);

///
/// A bounding box within a sprite
///
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SpriteBounds(pub SpritePosition, pub SpriteSize);
