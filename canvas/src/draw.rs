//!
//! Actions that can be performed to draw on a canvas
//!

use super::transform2d::*;
use super::texture::*;
use super::color::*;
use super::font::*;

///
/// Possible way to join lines
///
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum LineJoin {
    Miter,
    Round,
    Bevel
}

///
/// How to cap lines
///
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum LineCap {
    Butt,
    Round,
    Square
}

///
/// Blend mode to use when drawing
///
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum BlendMode {
    SourceOver,
    SourceIn,
    SourceOut,
    DestinationOver,
    DestinationIn,
    DestinationOut,
    SourceAtop,
    DestinationAtop,

    Multiply,
    Screen,
    Darken,
    Lighten
}

///
/// How a path should determine if it's an outer edge or not
///
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum WindingRule {
    /// A line is an outer edge if it's moving in the opposite direction to the 'inner edge' lines
    NonZero,

    /// Every line is an outer edge
    EvenOdd
}

///
/// Identifier of a canvas layer
///
/// Layers make it possible to re-draw part of a design without affecting the rest, which is particularly
/// useful for applications where different parts of the application are responsible for drawing different
/// parts of the canvas.
///
/// Layer rendering are usually cached, so they are also a good way to reduce the amount of time required
/// to do a redraw.
///
/// If a layer is cleared, other entities (such as sprites) are not affected, whereas `ClearCanvas` will
/// remove all entities from the canvas.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LayerId(pub u64);

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
/// Identifier for a font
///
/// Fonts can be used to render text: they need to be pre-loaded and are removed from the canvas by
/// `Draw::ClearCanvas`
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FontId(pub u64);

///
/// Identifier for a texture
///
/// Textures are bitmaps that can be used as fills
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TextureId(pub u64);

///
/// Transformation to apply to a canvas 'sprite'
///
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SpriteTransform {
    /// Resets the transformation to the identity transform
    Identity,

    /// Move by a particular amount
    Translate(f32, f32),

    /// Scale by the specified x and y factors about the origin
    Scale(f32, f32),

    /// Rotate by an angle in degrees about the origin
    Rotate(f32),

    /// Arbitrary 2D transformation
    Transform2D(Transform2D)
}

///
/// Instructions for drawing to a canvas
///
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum Draw {
    /// Begins a new path
    NewPath,

    /// Move to a new point
    Move(f32, f32),

    /// Line to point
    Line(f32, f32),

    /// Bezier curve to point
    BezierCurve((f32, f32), (f32, f32), (f32, f32)),

    /// Closes the current path
    ClosePath,

    /// Fill the current path
    Fill,

    /// Draw a line around the current path
    Stroke,

    /// Set the line width
    LineWidth(f32),

    /// Set the line width in pixels
    LineWidthPixels(f32),

    /// Line join
    LineJoin(LineJoin),

    /// The cap to use on lines
    LineCap(LineCap),

    /// Resets the dash pattern to empty (which is a solid line)
    NewDashPattern,

    /// Adds a dash to the current dash pattern
    DashLength(f32),

    /// Sets the offset for the dash pattern
    DashOffset(f32),

    /// Set the fill color
    FillColor(Color),

    /// Set the line color
    StrokeColor(Color),

    /// Set the winding rule for fill operations
    WindingRule(WindingRule),

    /// Set how future renderings are blended with one another
    BlendMode(BlendMode),

    /// Reset the transformation to the identity transformation
    IdentityTransform,

    /// Sets a transformation such that:
    /// (0,0) is the center point of the canvas
    /// (0,height/2) is the top of the canvas
    /// Pixels are square
    CanvasHeight(f32),

    /// Moves a particular region to the center of the canvas (coordinates are minx, miny, maxx, maxy)
    CenterRegion((f32, f32), (f32, f32)),

    /// Multiply a 2D transform into the canvas
    MultiplyTransform(Transform2D),

    /// Unset the clipping path
    Unclip,

    /// Clip to the currently set path
    Clip,

    /// Stores the content of the clipping path from the current layer in a background buffer
    Store,

    /// Restores what was stored in the background buffer. This should be done on the
    /// same layer that the Store operation was called upon.
    ///
    /// The buffer is left intact by this operation so it can be restored again in the future.
    ///
    /// (If the clipping path has changed since then, the restored image is clipped against the new path)
    Restore,

    /// Releases the buffer created by the last 'Store' operation
    ///
    /// Restore will no longer be valid for the current layer
    FreeStoredBuffer,

    /// Push the current state of the canvas (line settings, stored image, current path - all state)
    PushState,

    /// Restore a state previously pushed
    PopState,

    /// Clears the canvas entirely to a background colour, and removes any stored resources (layers, sprites, fonts, textures)
    ClearCanvas(Color),

    /// Selects a particular layer for drawing
    /// Layer 0 is selected initially. Layers are drawn in order starting from 0.
    /// Layer IDs don't have to be sequential.
    Layer(LayerId),

    /// Sets how a particular layer is blended with the underlying layer
    LayerBlend(LayerId, BlendMode),

    /// Clears the current layer
    ClearLayer,

    /// Selects a particular sprite for drawing
    ///
    /// Future drawing actions are sent to this sprite: use something like `Layer(0)` to start drawing
    /// to a layer again.
    ///
    /// Sprites can be repeatedly re-rendered with a single command and their appearance may be
    /// cached for efficiency. Actions that affect the whole canvas or layers are not permitted in
    /// sprites.
    Sprite(SpriteId),

    /// Releases the resources used by the current sprite
    ClearSprite,

    /// Adds a sprite transform to the current list of transformations to apply
    SpriteTransform(SpriteTransform),

    /// Renders a sprite with a set of transformations
    DrawSprite(SpriteId),

    /// Performs an operation on a texture
    Texture(TextureId, TextureOp),

    /// Performs an operation on a font
    Font(FontId, FontOp),

    /// Draws a string using a font with a baseline starting at the specified position
    DrawText(FontId, String, f32, f32)
}
