use crate::edgeplan::*;

use itertools::*;

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
fn resolve_shards(previous_line: &Vec<EdgeDescriptorIntercept>, next_line: &Vec<EdgeDescriptorIntercept>, shards: &mut Vec<ShardIntercept>) {
    struct InterceptIterator<'a, TIterator> {
        /// The shape that's being iterated over
        current_shape:      Option<usize>,

        /// True if current_intercept is at the start of a new shape
        new_shape:          bool,

        /// The first intercept for the current shape
        first_intercept:    Option<(&'a EdgeDescriptorIntercept, bool)>,

        /// The intercept last read from the iterator (next value to return)
        current_intercept:  Option<(&'a EdgeDescriptorIntercept, bool)>,

        /// Iterator of sorted intercepts without the 'loop' repetitions
        sorted_intercepts:  TIterator,         
    }

    impl<'a, TIterator> InterceptIterator<'a, TIterator> 
    where
        TIterator: Iterator<Item=&'a (&'a EdgeDescriptorIntercept, bool)>
    {
        pub fn new(iterator: TIterator) -> Self {
            let mut iterator        = iterator;
            let current_intercept   = iterator.next();

            InterceptIterator { 
                current_shape:      current_intercept.map(|intercept| intercept.0.position.0),
                new_shape:          false,
                first_intercept:    current_intercept.copied(), 
                current_intercept:  current_intercept.copied(), 
                sorted_intercepts:  iterator 
            }
        }
    }

    impl<'a, TIterator> Iterator for InterceptIterator<'a, TIterator> 
    where
        TIterator: Iterator<Item=&'a (&'a EdgeDescriptorIntercept, bool)>
    {
        type Item = (&'a EdgeDescriptorIntercept, bool);

        #[inline]
        fn next(&mut self) -> Option<(&'a EdgeDescriptorIntercept, bool)> {
            if self.new_shape {
                // At the end of each shape, return the first intercept again (because they loop around on themselves)
                self.new_shape          = false;

                // 'current_intercept' is the first item in the new shape at this point
                let result              = self.first_intercept;
                self.first_intercept    = self.current_intercept;

                result
            } else {
                // Fetch the next intercept and remove the current intercept
                let current_intercept   = self.current_intercept;
                let next_intercept      = if current_intercept.is_some() { self.sorted_intercepts.next() } else { None };

                if let Some(next_intercept) = next_intercept {
                    // Check if we've reached the end of the shape: we loop the intercept back on itself if true
                    let EdgePosition(shape_id, _, _) = next_intercept.0.position;
                    if self.current_shape != Some(shape_id) {
                        self.new_shape      = true;
                        self.current_shape  = Some(shape_id);
                    }
                } else {
                    // We've reached the end of the shape regardless
                    self.new_shape = true;
                }

                // Current intercept is always part of the current shape
                self.current_intercept = next_intercept.copied();
                current_intercept
            }
        }
    }

    // Clear out any existing values from the result
    shards.clear();

    // Mix the previous and next lines and then sort them by edge position
    let sorted_lines =
        previous_line.iter().map(|intercept| (intercept, false))
            .chain(next_line.iter().map(|intercept| (intercept, true)))
            .sorted_by(|(a, _), (b, _)| a.position.cmp(&b.position))
            .collect::<Vec<_>>();

    // When sorted this way, this puts 'connected' intercepts next to each other, so we can create shards from any pair where the first is on the lower edge 
    // and the second is on the upper edge, then sort again by x position. The shape is a loop, and so the ordering is too
    let mut last_matched                = false;
    let mut initial_subpath_intercept   = sorted_lines[0];

    // Current side can be None (unknown), true or false. We can only move from the 'true' side to the 'false' side
    let mut current_side                = None;

    for ((first_intercept, first_is_next), second) in InterceptIterator::new(sorted_lines.iter()).tuple_windows::<(_, _)>() {
        let (second_intercept, second_is_next) = if first_intercept.position.0 != second.0.position.0 {
            // Intercepts are on different shapes, so instead of using the original 'second' path, use the initial one from the current subpath
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

        if Some(second_is_next) == current_side {
            // Not transitioning from the 'current' side
            // This is rare and indicates a 'fault line' where the two sets of intercepts don't fully encapsulate the transitions between the two sides
            continue;
        }

        // Update the current side
        current_side = Some(second_is_next);

        /*
        if first_intercept.direction != second_intercept.direction {
            // Shouldn't happen?
            debug_assert!(false);
            continue;
        }
        */

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
    debug_assert!(previous_line.len()%2 == 0, "Previous line has odd number of intercepts: Previous line: {:?}\nNext line: {:?}\nSorted lines: {:?}\nShards found: {:?}", previous_line, next_line, InterceptIterator::new(sorted_lines.iter()).collect::<Vec<_>>(), shards);
    debug_assert!(next_line.len()%2 == 0, "Next line has odd number of intercepts: Previous line: {:?}\nNext line: {:?}\nSorted lines: {:?}\nShards found: {:?}", previous_line, next_line, InterceptIterator::new(sorted_lines.iter()).collect::<Vec<_>>(), shards);
    debug_assert!(sorted_lines.len()%2 == 0, "Sorted lines has odd number of intercepts: Previous line: {:?}\nNext line: {:?}\nSorted lines: {:?}\nShards found: {:?}", previous_line, next_line, InterceptIterator::new(sorted_lines.iter()).collect::<Vec<_>>(), shards);
    debug_assert!(shards.len()%2 == 0, "Previous line: {:?}\nNext line: {:?}\nSorted lines: {:?}\nShards found: {:?}", previous_line, next_line, InterceptIterator::new(sorted_lines.iter()).collect::<Vec<_>>(), shards);
}

///
/// Fills the output with a list of shard intercepts for a set positions
///
/// The start_y_positions, end_y_positions and output slices should be the same length. Each entry in the output slice will be cleared and
/// updated to contain the intercepts for the corresponding start/end region.
///
pub fn shard_intercepts_from_edge<'a, TEdge: EdgeDescriptor>(edge: &'a TEdge, start_y_positions: &'a [f64], end_y_positions: &'a [f64], output: &mut [Vec<ShardIntercept>]) {
    // TODO: iteration/check is only needed here because the caller doesn't guarantee a set of matched pairs (or even better: just a list of positions where each pair is a start and end position)
    // TODO: maybe even more performance to be had by using preallocated scratch space

    let mut intercepts;

    let (start_intercepts, end_intercepts) = if start_y_positions.iter().skip(1).zip(end_y_positions.iter().take(end_y_positions.len()-1)).all(|(a, b)| (a-b).abs() < 1e-7) {
        // Start positions and end positions are the same
        intercepts = vec![Vec::with_capacity(8); start_y_positions.len() + 1];

        // Common case where the start and end positions are the same (so we only need to compute one set)
        // TODO: if we only allowed homogenous start positions, could pass this in as a parameter
        let combined_positions = start_y_positions
            .iter()
            .copied()
            .chain(end_y_positions.last().into_iter().copied())
            .collect::<Vec<_>>();

        // Calculate one set of intercepts for both the start and end positions
        edge.intercepts(&combined_positions, &mut intercepts);
        intercepts.iter_mut()
            .for_each(|intercept_line| intercept_line.sort_by(|a, b| a.x_pos.total_cmp(&b.x_pos)));

        (&intercepts[0..start_y_positions.len()], &intercepts[1..start_y_positions.len()+1])
    } else {
        // Positions are non-homogenous
        intercepts = vec![Vec::with_capacity(8); start_y_positions.len() + end_y_positions.len()];
        let (start_intercepts, end_intercepts) = intercepts.split_at_mut(start_y_positions.len());

        // Read the positions of the start intercepts for each y-position
        edge.intercepts(start_y_positions, start_intercepts);

        // Read the end intercepts
        edge.intercepts(end_y_positions, end_intercepts);

        // Sort into intercept order
        start_intercepts.iter_mut()
            .for_each(|intercept_line| intercept_line.sort_by(|a, b| a.x_pos.total_cmp(&b.x_pos)));
        end_intercepts.iter_mut()
            .for_each(|intercept_line| intercept_line.sort_by(|a, b| a.x_pos.total_cmp(&b.x_pos)));

        (&*start_intercepts, &*end_intercepts)
    };

    // Generate the shart intercepts
    for ((previous_line, next_line), intercepts) in start_intercepts.into_iter().zip(end_intercepts.into_iter()).zip(output.iter_mut()) {
        // We now need to match the crossing points for the two lines, which we do by pairing up each point with the nearest of the same crossing type form the

        // Every matching pair forms a shard in that direction. Very often this is very simple: both the next and previous line have the same number of intercepts,
        // and they are all in the same direction
        if previous_line.len() == 0 || next_line.len() == 0 {
            // There are no shards in an empty line, so the other line doesn't matter (this is commonly the initial/final line for a convex shape)
            intercepts.clear();
        } else {
            // Shards are formed by finding the nearest intercept to each point
            // (Eg, the end of a spike in a concave shape)
            resolve_shards(&previous_line, &next_line, intercepts);
        }
    }
}
