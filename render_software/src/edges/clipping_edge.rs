use crate::edgeplan::*;

use std::sync::*;

///
/// A clipping region, defined by an edge description
///
pub struct ClipRegion<TEdge> 
where
    TEdge: EdgeDescriptor,
{
    /// The edges that make up the clip region
    region: Vec<TEdge>,
}

///
/// An edge that is clipped against another shape
///
pub struct ClippedShapeEdge<TEdge, TRegionEdge>
where
    TEdge:          EdgeDescriptor,
    TRegionEdge:    EdgeDescriptor,
{
    /// The ID of the shape
    shape_id: ShapeId,

    /// The region that this is clipped against
    region: Arc<ClipRegion<TRegionEdge>>,

    /// The edges making up the shape that should be clipped against the region
    shape_edges: Vec<TEdge>,
}

impl<TEdge> ClipRegion<TEdge>
where
    TEdge: EdgeDescriptor,
{
    ///
    /// Creates a new clipping region
    ///
    pub fn new(region: Vec<TEdge>) -> Self {
        ClipRegion { region }
    }
}

impl<TEdge, TRegionEdge> ClippedShapeEdge<TEdge, TRegionEdge>
where
    TEdge:          EdgeDescriptor,
    TRegionEdge:    EdgeDescriptor,
{
    ///
    /// Creates a new shape with a clipping region
    ///
    pub fn new(shape_id: ShapeId, region: Arc<ClipRegion<TRegionEdge>>, shape_edges: Vec<TEdge>) -> Self {
        ClippedShapeEdge {
            shape_id, region, shape_edges
        }
    }
}