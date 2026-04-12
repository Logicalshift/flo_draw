use crate::edgeplan::*;

use flo_canvas as canvas;
use smallvec::*;

use std::sync::*;

///
/// A clipping region, defined by an edge description
///
#[derive(Clone)]
pub struct ClipRegion<TEdge> 
where
    TEdge: EdgeDescriptor,
{
    /// The edges that make up the clip region
    region: Vec<TEdge>,

    /// The bounds of this clip region
    bounds: ((f64, f64), (f64, f64)),
}

///
/// An edge that is clipped against another shape
///
#[derive(Clone)]
pub struct ClippedShapeEdge<TEdge, TRegionEdge>
where
    TEdge:          Clone + EdgeDescriptor,
    TRegionEdge:    Clone + EdgeDescriptor,
{
    /// The ID of the shape
    shape_id: ShapeId,

    /// The region that this is clipped against
    region: Arc<ClipRegion<TRegionEdge>>,

    /// The edges making up the shape that should be clipped against the region
    shape_edges: Vec<TEdge>,

    /// The bounds of the shape (once this edge has been prepared for rendering)
    shape_bounds: ((f64, f64), (f64, f64)),
}

impl<TEdge> ClipRegion<TEdge>
where
    TEdge: 'static + EdgeDescriptor,
{
    ///
    /// Creates a new clipping region
    ///
    /// The edges should form a closed shape, and also have had `prepare_to_render()` called on them
    ///
    pub fn new(region: Vec<TEdge>) -> Self {
        // Calculate the bounds of the clip region from the edges
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;

        for edge in region.iter() {
            let ((edge_minx, edge_miny), (edge_maxx, edge_maxy)) = edge.bounding_box();

            min_x = min_x.min(edge_minx);
            min_y = min_y.min(edge_miny);
            max_x = max_x.max(edge_maxx);
            max_y = max_y.max(edge_maxy);
        }

        let bounds = ((min_x, min_y), (max_x, max_y));

        ClipRegion { 
            region,
            bounds
        }
    }

    ///
    /// Converts this clip region to use an 'object' style of edge
    ///
    pub fn to_object(self) -> ClipRegion<Arc<dyn EdgeDescriptor>> {
        ClipRegion {
            region: self.region.into_iter().map(|edge| { let edge: Arc<dyn EdgeDescriptor> = Arc::new(edge); edge }).collect(),
            bounds: self.bounds,
        }
    }

    ///
    /// Applies a transformation to this region
    ///
    fn transform(&self, transform: &canvas::Transform2D) -> ClipRegion<Arc<dyn EdgeDescriptor>> {
        // Transform the edges in the region
        let region = self.region.iter()
            .map(|edge| edge.transform(transform))
            .collect::<Vec<_>>();

        // Recompute the bounding box
        let bounds = region.iter()
            .fold(((f64::MAX, f64::MAX), (f64::MIN, f64::MIN)), |((xa1, ya1), (xa2, ya2)), edge| {
                let ((xb1, yb1), (xb2, yb2)) = edge.bounding_box();

                ((xa1.min(xb1), ya1.min(yb1)), (xa2.max(xb2), ya2.max(yb2)))
            });

        ClipRegion {
            region,
            bounds
        }
    }
}

impl<TEdge, TRegionEdge> ClippedShapeEdge<TEdge, TRegionEdge>
where
    TEdge:          Clone + EdgeDescriptor,
    TRegionEdge:    Clone + EdgeDescriptor,
{
    ///
    /// Creates a new shape with a clipping region
    ///
    /// For the clipping algorithm to work properly, we need a complete closed shape and not just individual edges.
    ///
    pub fn new(shape_id: ShapeId, region: Arc<ClipRegion<TRegionEdge>>, shape_edges: Vec<TEdge>) -> Self {
        // The clip region bounds will be larger or the same as the bounds for the resulting edge
        ClippedShapeEdge {
            shape_bounds:   region.bounds,
            shape_id:       shape_id, 
            region:         region, 
            shape_edges:    shape_edges,
        }
    }
}

impl<TEdge, TRegionEdge> EdgeDescriptor for ClippedShapeEdge<TEdge, TRegionEdge>
where
    TEdge:          'static + Clone + EdgeDescriptor,
    TRegionEdge:    'static + Clone + EdgeDescriptor,
{
    fn clone_as_object(&self) -> Arc<dyn EdgeDescriptor> {
        Arc::new(self.clone())
    }

    fn prepare_to_render(&mut self) {
        // Prepare the edges for rendering
        self.shape_edges.iter_mut().for_each(|edge| edge.prepare_to_render());

        // Calculate the bounds of the shape region from the edges
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;

        for edge in self.shape_edges.iter() {
            let ((edge_minx, edge_miny), (edge_maxx, edge_maxy)) = edge.bounding_box();

            min_x = min_x.min(edge_minx);
            min_y = min_y.min(edge_miny);
            max_x = max_x.max(edge_maxx);
            max_y = max_y.max(edge_maxy);
        }

        // The shape bounds are constrained by the clipping region bounds
        let ((clip_minx, clip_miny), (clip_maxx, clip_maxy)) = self.region.bounds;
        self.shape_bounds = ((min_x.max(clip_minx), min_y.max(clip_miny)), (max_x.min(clip_maxx), max_y.min(clip_maxy)));
    }

    #[inline]
    fn shape(&self) -> ShapeId {
        self.shape_id
    }

    fn bounding_box(&self) -> ((f64, f64), (f64, f64)) {
        self.shape_bounds
    }

    fn transform(&self, transform: &canvas::Transform2D) -> Arc<dyn EdgeDescriptor> {
        // TODO: if the region is shared we really should continue to share it amongst all the transformed shapes
        let region      = Arc::new(self.region.transform(transform));
        let shape_edges = self.shape_edges.iter().map(|edge| edge.transform(transform)).collect();

        let mut new_edge  = ClippedShapeEdge {
            shape_id:       self.shape_id,
            region:         region,
            shape_edges:    shape_edges,
            shape_bounds:   ((0.0, 0.0), (0.0, 0.0)),
        };
        new_edge.prepare_to_render();

        Arc::new(new_edge)
    }

    fn intercepts(&self, y_positions: &[f64], output: &mut [Vec<EdgeDescriptorIntercept>]) {
        // Collect the clipping range for these y positions
        // TODO: often we'll clip against multiple shapes for the same set of y coordinates, so a way to cache these results would speed things up
        // TODO: need to use unique subpath IDs when returning different edges
        let mut clip_intercepts = vec![vec![]; y_positions.len()];

        // Append the edges from each of the shapes making up the clip region
        for clip_edge in self.region.region.iter() {
            clip_edge.intercepts(y_positions, &mut clip_intercepts);
        }

        // Sort the intercepts so we can trace the clipping region from left to right
        for intercept_list in clip_intercepts.iter_mut() {
            intercept_list.sort_by(|a, b| a.x_pos.total_cmp(&b.x_pos));
        }

        // Collect the unclipped versions of the shape edges
        let mut unclipped_shape = vec![vec![]; y_positions.len()];
        for shape_edge in self.shape_edges.iter() {
            shape_edge.intercepts(y_positions, &mut unclipped_shape);
        }

        // Sort the intercepts so we can trace the clipping region from left to right
        for intercept_list in unclipped_shape.iter_mut() {
            intercept_list.sort_by(|a, b| a.x_pos.total_cmp(&b.x_pos));
        }

        // Typically there are only a small number of overlaps so we don't need to allocate on the heap for the current shape
        let mut current_shape_edge: SmallVec<[EdgeDescriptorIntercept; 8]> = smallvec![];

        // Perform clipping for each line we're processing
        for (_y_pos, (clip_edge, (unclipped_shape, output))) in y_positions.iter().zip(clip_intercepts.into_iter().zip(unclipped_shape.into_iter().zip(output.iter_mut()))) {
            // Clear out the current shape
            current_shape_edge.clear();

            // Non-zero if we're inside the clipping shape (should generate output)
            let mut clip_inside;

            // Iterate over the clipping shape
            let mut clip_iter       = clip_edge.iter();

            // Iterate over the current shape, too
            let mut shape_inside        = 0;
            let mut shape_iter          = unclipped_shape.iter();
            let Some(mut shape_next)    = shape_iter.next() else { continue; };

            'clip_region: loop {
                // Read the next region from the clipping iterator
                let Some(region_start) = clip_iter.next() else { break; };

                clip_inside = match region_start.direction {
                    EdgeInterceptDirection::ToggleOut       => 1,
                    EdgeInterceptDirection::ToggleIn        => 1,
                    EdgeInterceptDirection::DirectionOut    => 1,
                    EdgeInterceptDirection::DirectionIn     => -1,
                };

                let mut region_end = region_start;

                while clip_inside != 0 {
                    let Some(next_region) = clip_iter.next() else { break 'clip_region; };

                    clip_inside = match next_region.direction {
                        EdgeInterceptDirection::ToggleOut       => 0,
                        EdgeInterceptDirection::ToggleIn        => 0,
                        EdgeInterceptDirection::DirectionOut    => clip_inside + 1,
                        EdgeInterceptDirection::DirectionIn     => clip_inside - 1,
                    };

                    region_end = next_region;
                }

                // Process shape items until they leave the region
                let clip_region = region_start.x_pos..region_end.x_pos;

                // Process shape entries to the left of the clip region
                while shape_next.x_pos < clip_region.start {
                    // Determine whether or not we're inside the shape following this item
                    shape_inside = match shape_next.direction {
                        EdgeInterceptDirection::ToggleOut       |
                        EdgeInterceptDirection::ToggleIn        => if shape_inside != 0 { 0 } else { 1 },
                        EdgeInterceptDirection::DirectionOut    => shape_inside + 1,
                        EdgeInterceptDirection::DirectionIn     => shape_inside - 1,
                    };

                    // The clipped edges 'build up' in the current shape edge (if the 'current' edge overlaps the clipping region we need to replay them all to get into the right state)
                    if shape_inside == 0 {
                        current_shape_edge.clear();
                    } else {
                        current_shape_edge.push(*shape_next);
                    }

                    // Move on (or give up if we run out of shape edges)
                    shape_next = if let Some(next) = shape_iter.next() { next } else { break 'clip_region; };
                }

                // Next intercept is inside the shape or overruns it
                // Fill up the output with the transitions that make up the current region, bunched together at the start of the clip region (there'll only be more than one if the shape overlaps itself)
                output.extend(current_shape_edge.iter().map(|edge|
                    EdgeDescriptorIntercept {
                        x_pos:      clip_region.start,
                        direction:  edge.direction,
                        position:   edge.position,
                    }
                ));

                // Process shape entries inside the shape. We still keep track of the 'current edge' for regions that extend outside of the clip bounds
                while shape_next.x_pos < clip_region.end {
                    // These go to the output unchanged
                    output.push(*shape_next);

                    // Determine whether or not we're inside the shape following this item
                    shape_inside = match shape_next.direction {
                        EdgeInterceptDirection::ToggleOut       |
                        EdgeInterceptDirection::ToggleIn        => if shape_inside != 0 { 0 } else { 1 },
                        EdgeInterceptDirection::DirectionOut    => shape_inside + 1,
                        EdgeInterceptDirection::DirectionIn     => shape_inside - 1,
                    };

                    // The clipped edges 'build up' in the current shape edge (if the 'current' edge overlaps the clipping region we need to replay them all to get into the right state)
                    if shape_inside == 0 {
                        current_shape_edge.clear();
                    } else {
                        current_shape_edge.push(*shape_next);
                    }

                    // Move on (or give up if we run out of shape edges)
                    shape_next = if let Some(next) = shape_iter.next() { next } else { break 'clip_region; };
                }

                // 'Unwind' the edge so we leave the clipping region
                output.extend(current_shape_edge.iter().rev().map(|edge| {
                    EdgeDescriptorIntercept {
                        x_pos:      clip_region.end,
                        direction:  edge.direction.opposite(),
                        position:   edge.position,
                    }
                }));

                // Carry on here, we're looking for the next clip region
                // TODO: I think one issue here is that if there are multiple clip regions the edge positions aren't distinct
                // We need a way to change the shape IDs when we re-enter the clip region so they're unique (this isn't seen unless the clip region is re-entered on the same line)
            }
        }

        /*
        // Clip the shape by scanning the clipping intercepts
        for y_idx in 0..y_positions.len() {
            // The crossing count for the clipping shape (0 = outside shape, non-zero = inside shape)
            let mut clip_inside = 0;

            // Look ahead in the list of clipping intercepts, ie looking for the next point where the clipping changes
            let mut clip_iter   = clip_intercepts[y_idx].iter();
            let mut clip_next   = if let Some(next) = clip_iter.next() { 
                next
            } else {
                // This entire line is clipped away
                continue;
            };

            // Iterate across the shape
            let mut shape_inside    = 0;
            let mut shape_iter      = unclipped_shape[y_idx].iter();
            let output              = &mut output[y_idx];

            'clip_region: while let Some(intercept) = shape_iter.next() {
                let shape_dir = &intercept.direction;
                let shape_pos = &intercept.x_pos;

                // Advance the 'clip_next' position until it is after the current state
                while clip_next.x_pos < *shape_pos {
                    // Update the 'inside' part of the clipping rectangle
                    let was_inside  = clip_inside != 0;
                    clip_inside     = match clip_next.direction {
                        EdgeInterceptDirection::ToggleIn        |
                        EdgeInterceptDirection::ToggleOut       => if clip_inside == 0 { 1 } else { 0 },
                        EdgeInterceptDirection::DirectionIn     => clip_inside + 1,
                        EdgeInterceptDirection::DirectionOut    => clip_inside - 1,
                    };
                    let is_inside   = clip_inside != 0;

                    // Enter/leave the shape if we're inside it already
                    if shape_inside != 0 && was_inside != is_inside {
                        let direction = match clip_next.direction {
                            EdgeInterceptDirection::DirectionIn     => EdgeInterceptDirection::ToggleIn,
                            EdgeInterceptDirection::DirectionOut    => EdgeInterceptDirection::ToggleOut,
                            other                                   => other,
                        };

                        output.push(EdgeDescriptorIntercept { direction: direction, x_pos: clip_next.x_pos, position: intercept.position })
                    }

                    // Move to the next clip intercept
                    clip_next = if let Some(next) = clip_iter.next() {
                        next
                    } else {
                        // Once there are no more clip intercepts, the remaining points are all outside
                        break 'clip_region;
                    };
                }

                // Update whether or not we're inside the shape
                let was_inside  = shape_inside != 0;
                shape_inside    = match shape_dir {
                    EdgeInterceptDirection::ToggleIn        |
                    EdgeInterceptDirection::ToggleOut       => if shape_inside == 0 { 1 } else { 0 },
                    EdgeInterceptDirection::DirectionIn     => shape_inside + 1,
                    EdgeInterceptDirection::DirectionOut    => shape_inside - 1,
                };
                let is_inside   = shape_inside != 0;

                // clip_next is the closest following clip region to the current shape, so clip_inside can be used to determine if this point is inside the shape
                if clip_inside != 0 && was_inside != is_inside {
                    let direction = match *shape_dir {
                        EdgeInterceptDirection::DirectionIn     => EdgeInterceptDirection::ToggleIn,
                        EdgeInterceptDirection::DirectionOut    => EdgeInterceptDirection::ToggleOut,
                        other                                   => other,
                    };

                    output.push(EdgeDescriptorIntercept { direction: direction, x_pos: *shape_pos, position: intercept.position });
                }
            }
        }
        */

        #[cfg(debug_assertions)]
        {
            for y_idx in 0..y_positions.len() {
                debug_assert!(output[y_idx].len()%2 == 0, "Clipping: non-zero intercepts, {:?}, y={}", output[y_idx], y_positions[y_idx]);
            }
        }
    }

    fn description(&self) -> String {
        use itertools::*;
        format!("Clipped edge: {}", self.shape_edges.iter().map(|edge| edge.description()).join(", "))
    }

    fn apexes(&self, output: &mut Vec<f64>) {
        // Use the apexes from all of the edges in the shape
        for edge in self.shape_edges.iter() {
            edge.apexes(output);
        }

        // Plus the apexes from the clipping path
        for edge in self.region.region.iter() {
            edge.apexes(output);
        }
    }
}