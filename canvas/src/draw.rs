//!
//! Actions that can be performed to draw on a canvas
//!

use super::transform2d::*;
use super::color::*;

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
/// Instructions for drawing to a canvas
///
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
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

    /// Clears the canvas entirely
    ClearCanvas,

    /// Selects a particular layer for drawing
    /// Layer 0 is selected initially. Layers are drawn in order starting from 0.
    /// Layer IDs don't have to be sequential.
    Layer(u32),

    /// Sets how a particular layer is blended with the underlying layer
    LayerBlend(u32, BlendMode),

    /// Clears the current layer
    ClearLayer
}
