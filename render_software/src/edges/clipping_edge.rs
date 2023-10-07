use crate::edgeplan::*;

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
    TEdge: EdgeDescriptor,
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
            max_x = max_x.min(edge_maxx);
            max_y = max_y.min(edge_maxy);
        }

        let bounds = ((min_x, min_y), (max_x, max_y));

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
            max_x = max_x.min(edge_maxx);
            max_y = max_y.min(edge_maxy);
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

    fn intercepts(&self, y_positions: &[f64], output: &mut [smallvec::SmallVec<[(EdgeInterceptDirection, f64); 2]>]) {
        // Collect the clipping range for these y positions
        // TODO: often we'll clip against multiple shapes for the same set of y coordinates, so a way to cache these results would speed things up
        let mut clip_intercepts = vec![smallvec![]; y_positions.len()];

        // Append the edges from each of the shapes making up the clip region
        for clip_edge in self.region.region.iter() {
            clip_edge.intercepts(y_positions, &mut clip_intercepts);
        }

        // Sort the intercepts so we can trace the clipping region from left to right
        for intercept_list in clip_intercepts.iter_mut() {
            intercept_list.sort_by(|(_, a_xpos), (_, b_xpos)| a_xpos.total_cmp(b_xpos));
        }

        // Collect the unclipped versions of the shape edges
        for shape_edge in self.shape_edges.iter() {
            shape_edge.intercepts(y_positions, output);
        }

        // Sort the intercepts so we can trace the clipping region from left to right
        for intercept_list in clip_intercepts.iter_mut() {
            intercept_list.sort_by(|(_, a_xpos), (_, b_xpos)| a_xpos.total_cmp(b_xpos));
        }

        // TODO: clip the intercepts

        todo!()
    }
}