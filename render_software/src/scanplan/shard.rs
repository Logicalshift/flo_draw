use crate::edgeplan::*;

use smallvec::*;

use std::ops::{Range};

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

impl ShardIntercept {
    ///
    /// The direction of this intercept
    ///
    #[inline]
    pub fn direction(&self) -> EdgeInterceptDirection {
        self.direction
    }

    ///
    /// The x-range covered by this intercept
    ///
    #[inline]
    pub fn x_range(&self) -> Range<f64> {
        self.x_start..self.x_end
    }
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
        // Fetch the following line. The preceding line was sorted by the last pass through this routine
        let previous_line   = &self.previous_line;
        let mut next_line   = if let Some(next_line) = self.intercepts.next() { next_line } else { return None; };

        // Sort into order so we can match the two lines against each other
        next_line.sort_by(|(_, x1), (_, x2)| x1.total_cmp(x2));

        // We now need to match the crossing points for the two lines, which we do by pairing up each point with the nearest of the same crossing type form the

        // Every matching pair forms a shard in that direction. Very often this is very simple: both the next and previous line have the same number of intercepts,
        // and they are all in the same direction
        let mut intercepts;

        if previous_line.len() == 0 || next_line.len() == 0 {
            // There are no shards in an empty line, so the other line doesn't matter (this is commonly the initial/final line for a convex shape)
            intercepts = smallvec![];
        } else if previous_line.len() == next_line.len() {
            // Try the simple case, and then try finding the nearest matches if it fails
            intercepts = smallvec![];

            for (first, second) in previous_line.iter().zip(next_line.iter()) {
                if first.0 != second.0 {
                    // Intercept directions changed, so these shapes don't match: use the 'find nearest' algorithm instead (this is a concave shape)
                    // (Eg: a 'C' shape with a very narrow gap)
                    todo!()
                }

                // Add a new intercept to the list
                intercepts.push(ShardIntercept {
                    direction:  first.0,
                    x_start:    first.1.min(second.1),
                    x_end:      first.1.max(second.1),
                })
            }
        } else {
            // Shards are formed by finding the nearest intercept to each point
            // (Eg, the end of a spike in a concave shape)
            todo!()
        }

        // The next line now becomes the previous line
        self.previous_line = next_line;

        Some(intercepts)
    }
}

///
/// Creates an iterator that finds all of the shard intercepts across a range of y values
///
/// There will be one less line returned here than y-values that were passed in. Intercepts are ordered by x position on return.
///
pub fn shard_intercepts_from_edge<'a, TEdge: EdgeDescriptor>(edge: &'a TEdge, y_positions: &'a [f64]) -> impl 'a + Iterator<Item=SmallVec<[ShardIntercept; 2]>>{
    // Allocate space for the intercepts
    let mut intercepts = vec![smallvec![]; y_positions.len()];

    // Read the intercepts from the edge
    edge.intercepts(y_positions, &mut intercepts);

    // Read through the intercepts
    let mut intercepts  = intercepts.into_iter();
    let mut first_line  = intercepts.next().expect("Must be at least one y-position to generate a shard iterator");

    first_line.sort_by(|(_, x1), (_, x2)| x1.total_cmp(x2));

    // Create the shard iterator
    ShardIterator {
        previous_line:  first_line,
        intercepts:     intercepts
    }
}
