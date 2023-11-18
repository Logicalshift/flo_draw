use crate::edgeplan::*;

use smallvec::*;

///
/// A shard represents an interception with an edge over a range of y-values (generally a single pixel in height)
///
/// This type of intercept is called a shard as it represents a wedge where at one extreme the shape does not overlap
/// the image at all, and at other other it overlaps it 100%. The `LinearSourceOver` PixelProgramPlan can be used to
/// create an anti-aliasing effect where the shape is faded in across a scanline. 
///
/// When entering a shape the start position has an opacity of 0% with the end position being where the shape is 100% 
/// opaque, and when leaving a shape the reverse is true.
///
/// For very thin shapes, 'entry' and 'exit' shards may overlap.
///
/// Shards are an appoximation: they assume that the shape is locally flat, and it is possible to construct concave
/// shapes that can confuse the algorithm. Additionally, this won't add pixels for very long thin spikes.
///
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ShardIntercept {
    /// The direction of this intercept
    direction: EdgeInterceptDirection,

    /// The lower x value where this intercept starts (the point where the shard starts)
    x_start: f64,

    /// The upper x value where this intercept ends (the point where the shard ends)
    x_end: f64,
}

///
/// Processes normal intercepts into shards
///
struct ShardIterator<TInterceptIterator> 
where
    TInterceptIterator: Iterator<Item=SmallVec<[(EdgeInterceptDirection, f64); 2]>>
{
    /// The intercepts on the preceding line for this shape
    previous_line:  SmallVec<[(EdgeInterceptDirection, f64); 2]>,

    /// The intercepts across the line range to generate shards for
    intercepts:     TInterceptIterator,
}

impl<TInterceptIterator> Iterator for ShardIterator<TInterceptIterator>
where
    TInterceptIterator: Iterator<Item=SmallVec<[(EdgeInterceptDirection, f64); 2]>>
{
    type Item = SmallVec<[ShardIntercept; 2]>;

    fn next(&mut self) -> Option<SmallVec<[ShardIntercept; 2]>> {
        todo!()
    }
}

///
/// Creates an iterator that finds all of the shard intercepts across a range of y values
///
/// There will be one less line returned here than y-values that were passed in
///
pub fn shard_intercepts_from_edge<'a, TEdge: EdgeDescriptor>(edge: &'a TEdge, y_positions: &'a [f64]) -> impl 'a + Iterator<Item=SmallVec<[ShardIntercept; 2]>>{
    // Allocate space for the intercepts
    let mut intercepts = vec![smallvec![]; y_positions.len()];

    // Read the intercepts from the edge
    edge.intercepts(y_positions, &mut intercepts);

    // Read through the intercepts
    let mut intercepts  = intercepts.into_iter();
    let first_line      = intercepts.next().expect("Must be at least one y-position to generate a shard iterator");

    // Create the shard iterator
    ShardIterator {
        previous_line:  first_line,
        intercepts:     intercepts
    }
}
