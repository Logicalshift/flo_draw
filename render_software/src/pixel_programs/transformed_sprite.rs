use super::basic_sprite::*;

use crate::edgeplan::*;
use crate::pixel::*;
use crate::scanplan::*;

use flo_canvas as canvas;

use std::marker::{PhantomData};
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
    basic_program:      PhantomData<BasicSpriteProgram<TPixel, Arc<dyn EdgeDescriptor>, TPlanner>>,
    edge_descriptor:    PhantomData<EdgePlan<TEdgeDescriptor>>
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

impl<TPixel, TEdgeDescriptor, TPlanner> Default for TransformedSpriteProgram<TPixel, TEdgeDescriptor, TPlanner> 
where
    TEdgeDescriptor:    'static + EdgeDescriptor,
    TPixel:             'static,
{
    fn default() -> Self {
        TransformedSpriteProgram {
            basic_program:      PhantomData, 
            edge_descriptor:    PhantomData,
        }
    }
}

impl<TEdgeDescriptor> TransformedSpriteData<TEdgeDescriptor>
where
    TEdgeDescriptor: EdgeDescriptor,
{
    ///
    /// Creates a new data object for the transformed sprite program
    ///
    pub fn new(edges: Arc<EdgePlan<TEdgeDescriptor>>, transform: canvas::Transform2D) -> Self {
        TransformedSpriteData {
            edges:      RwLock::new(TransformedEdges::OriginalEdges(edges)),
            transform:  transform,
        }
    }
}

impl<TPixel, TEdgeDescriptor, TPlanner> PixelProgramForFrame for TransformedSpriteProgram<TPixel, TEdgeDescriptor, TPlanner>
where
    TEdgeDescriptor:    'static + EdgeDescriptor,
    TPixel:             'static + Copy + Send + Sync + AlphaBlend,
    TPlanner:           Send + Sync + Default + ScanPlanner<Edge=Arc<dyn EdgeDescriptor>>,
{
    /// The type of the pixel program that this will run
    type Program = BasicSpriteProgram<TPixel, Arc<dyn EdgeDescriptor>, TPlanner>;

    ///
    /// The data that is associated with an instance of this program (can generate the data required for the pixel program itself)
    ///
    type FrameData = TransformedSpriteData<TEdgeDescriptor>;

    ///
    /// Creates a pixel program and the corresponding data that will run for a given frame size
    ///
    fn program_for_frame(&self, _pixel_size: PixelSize, program_data: &Arc<Self::FrameData>) -> (BasicSpriteProgram<TPixel, Arc<dyn EdgeDescriptor>, TPlanner>, BasicSpriteData<Arc<dyn EdgeDescriptor>>) {
        let transformed_edges = {
            let mut edges = program_data.edges.write().unwrap();

            match &*edges {
                TransformedEdges::OriginalEdges(original_edges) => {
                    // Transform the edegs and update the state
                    let transformed_edges = Arc::new(original_edges.transform(&program_data.transform));
                    *edges = TransformedEdges::TransformedEdges(transformed_edges.clone());

                    transformed_edges
                }

                TransformedEdges::TransformedEdges(transformed_edges) => {
                    // Just use the existing edges
                    Arc::clone(transformed_edges)
                }
            }
        };

        // Program is a basic sprite program
        let program = BasicSpriteProgram::default();

        // Data uses the edges we retrieved, with no linear transform
        let sprite_data = BasicSpriteData::new(transformed_edges, (1.0, 1.0), (0.0, 0.0));

        (program, sprite_data)
    }
}
