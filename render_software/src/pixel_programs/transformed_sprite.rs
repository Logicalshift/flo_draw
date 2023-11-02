use super::basic_sprite::*;

use crate::edgeplan::*;
use crate::pixel::*;
use crate::scanplan::*;
use crate::render::*;

use flo_canvas as canvas;

use std::ops::{Range};
use std::sync::*;

///
/// The edges supplied to the transformed sprite data, either the version after transformation or the version before
///
enum TransformedEdges<TEdgeDescriptor>
where
    TEdgeDescriptor: EdgeDescriptor,
{
    /// The original set of edges, before they've been transformed
    OriginalEdges(Arc<EdgePlan<TEdgeDescriptor>>),

    /// The transformed set of edges
    TransformedEdges(Arc<EdgePlan<Arc<dyn EdgeDescriptor>>>)
}

///
/// A sprite is a renderer that can be run as a pixel program. This can be used for repeatedly re-rendering a shape
/// with a performance boost from bypassing the need to perform many of the usual preparation steps.
///
/// Sprite programs are generally drawn as transparent so they can blend with the pixels underneath but can potentially
/// be rendered more efficiently if the algorithm is able to detect opaque areas.
///
pub struct TransformedSpriteProgram<TPixel, TEdgeDescriptor, TPlanner>
where
    TEdgeDescriptor:    'static + EdgeDescriptor,
    TPixel:             'static,
{
    /// The basic program that will do the work for this program
    basic_program: BasicSpriteProgram<TPixel, TEdgeDescriptor, TPlanner>,
}

///
/// Data that can be used to run a basic sprite program
///
pub struct TransformedSpriteData<TEdgeDescriptor>
where
    TEdgeDescriptor: EdgeDescriptor,
{
    /// The untransformed edges for this sprite
    edges: RwLock<TransformedEdges<TEdgeDescriptor>>,

    /// The transform that will be applied to this prorgam
    transform: canvas::Transform2D,
}
