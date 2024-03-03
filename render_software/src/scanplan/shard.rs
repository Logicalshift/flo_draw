use crate::edgeplan::*;

use itertools::*;
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
/// For a set of shards where the previous line does not match up with the next line, uses the edge indexes to figure out what the shard intercepts must be
///
/// We short-cut this when the intercepts on the previous and next line match up (same number and same direction), so this is onyl used when there isn't an
/// uninterrupted slice.
///
/// We don't find maxima for peaks or minima for troughs, so one artifact this will introduce is that the subpixel peak or trough of a shape will be cut off.
///
fn resolve_shards(previous_line: &SmallVec<[EdgeDescriptorIntercept; 2]>, next_line: &SmallVec<[EdgeDescriptorIntercept; 2]>) -> SmallVec<[ShardIntercept; 2]> {
    // Mix the previous and next lines and then sort them by edge position
    let mut sorted_lines =
        previous_line.iter().map(|intercept| (intercept, false))
            .chain(next_line.iter().map(|intercept| (intercept, true)))
            .sorted_by(|(a, _), (b, _)| a.position.cmp(&b.position))
            .collect::<Vec<_>>();

    // The shape is a loop, so push the first element back on to the end
    if let Some(first) = sorted_lines.get(0) {
        sorted_lines.push(*first);
    }

    // When sorted this way, this puts 'connected' intercepts next to each other, so we can create shards from any pair where the first is on the lower edge 
    // and the second is on the upper edge, then sort again by x position. The shape is a loop, and so the ordering is too
    let mut shards                      = smallvec![];
    let mut last_matched                = false;
    let mut initial_subpath_intercept   = &sorted_lines[0];

    for ((first_intercept, first_is_next), second) in sorted_lines.iter().tuple_windows::<(_, _)>() {
        let (second_intercept, second_is_next) = if first_intercept.position.0 != second.0.position.0 {
            // Intercepts are on different, so instead of using the original 'second' path, use the initial one from the current subpath
            let result = initial_subpath_intercept;

            initial_subpath_intercept = second;

            result
        } else {
            second
        };

        if last_matched {
            // Don't use the same intercept in two shards
            last_matched = false;
            continue;
        }

        if first_is_next == second_is_next {
            // Both intercepts are on the same line, so don't form a shard
            continue;
        }

        if first_intercept.direction != second_intercept.direction {
            // Shouldn't happen?
            continue;
        }

        // The first intercept is on opposite line to the second intercept, indicating that the shape crossed inbetween the two lines
        let shard = ShardIntercept {
            direction:  first_intercept.direction,
            x_start:    first_intercept.x_pos.min(second_intercept.x_pos),
            x_end:      first_intercept.x_pos.max(second_intercept.x_pos),
        };

        shards.push(shard);
        last_matched = true;
    }

    // For a closed shape, there should always be an even number of intercepts, even after this transformation
    debug_assert!(shards.len()%2 == 0, "Previous line: {:?}\nNext line: {:?}\nSorted lines: {:?}\nShards found: {:?}", previous_line, next_line, sorted_lines, shards);

    shards
}

///
/// Creates an iterator that finds all of the shard intercepts across a range of y values
///
/// The start_y_positions and end_y_positions slices should be the same length. A slice will be generated for each pair of y values in these
/// two slices.
///
pub fn shard_intercepts_from_edge<'a, TEdge: EdgeDescriptor>(edge: &'a TEdge, start_y_positions: &'a [f64], end_y_positions: &'a [f64]) -> impl 'a + Iterator<Item=SmallVec<[ShardIntercept; 2]>>{
    // TODO: some edges can have multiple closed shapes (eg: closed lines, for example). This algorithm won't work with those because it assumes a single closed shape

    // Read the positions of the start intercepts for each y-position
    let mut start_intercepts = vec![smallvec![]; start_y_positions.len()];
    edge.intercepts(start_y_positions, &mut start_intercepts);

    start_intercepts.iter_mut()
        .for_each(|intercept_line| intercept_line.sort_by(|a, b| a.x_pos.total_cmp(&b.x_pos)));

    // Read the end intercepts (TODO: can maybe speed this up and only read the last one as very often end_y_positions[x] = start_y_positions[x+1])
    let mut end_intercepts = vec![smallvec![]; end_y_positions.len()];
    edge.intercepts(end_y_positions, &mut end_intercepts);

    // TODO: can avoid sorting things that we already fetched with the start intercepts
    end_intercepts.iter_mut()
        .for_each(|intercept_line| intercept_line.sort_by(|a, b| a.x_pos.total_cmp(&b.x_pos)));

    // Generate the shart intercepts
    start_intercepts.into_iter()
        .zip(end_intercepts.into_iter())
        .map(|(previous_line, next_line)| {
            // We now need to match the crossing points for the two lines, which we do by pairing up each point with the nearest of the same crossing type form the

            // Every matching pair forms a shard in that direction. Very often this is very simple: both the next and previous line have the same number of intercepts,
            // and they are all in the same direction
            let mut intercepts;

            if previous_line.len() == 0 || next_line.len() == 0 {
                // There are no shards in an empty line, so the other line doesn't matter (this is commonly the initial/final line for a convex shape)
                intercepts = smallvec![];
            } else if previous_line.len() == next_line.len() && false {
                // Try the simple case, and then try finding the nearest matches if it fails
                intercepts = smallvec![];

                for (first, second) in previous_line.iter().zip(next_line.iter()) {
                    if first.direction != second.direction || first.position.0 != second.position.0 {
                        // Intercept direction or shape has changed, so these shapes don't match: use the 'find nearest' algorithm instead (this is a concave shape)
                        // (Eg: a 'C' shape with a very narrow gap)
                        intercepts = resolve_shards(&previous_line, &next_line);
                        break;
                    }

                    // Add a new intercept to the list
                    intercepts.push(ShardIntercept {
                        direction:  first.direction,
                        x_start:    first.x_pos.min(second.x_pos),
                        x_end:      first.x_pos.max(second.x_pos),
                    })
                }
            } else {
                // Shards are formed by finding the nearest intercept to each point
                // (Eg, the end of a spike in a concave shape)
                intercepts = resolve_shards(&previous_line, &next_line);
            }

            intercepts
        })
}
