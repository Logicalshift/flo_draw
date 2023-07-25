use super::edge_descriptor::*;
use super::shape_descriptor::*;
use super::shape_id::*;

use std::collections::{HashMap};

///
/// An edge plan describes a 2 dimensional space as a set of edges that divide 
///
pub struct EdgePlan<TEdge>
where
    TEdge: EdgeDescriptor,
{
    /// Describes the shapes
    shapes: HashMap<ShapeId, ShapeDescriptor>,

    /// The edges themselves
    edges: Vec<TEdge>,
}

impl<TEdge> EdgePlan<TEdge>
where
    TEdge: EdgeDescriptor,
{
    ///
    /// Creates an empty edge plan
    ///
    pub fn new() -> EdgePlan<TEdge> {
        EdgePlan {
            shapes: HashMap::new(),
            edges:  vec![],
        }
    }

    ///
    /// Stores the details of how the interior of a shape should be rendered
    ///
    pub fn declare_shape_description(&mut self, shape_id: ShapeId, descriptor: ShapeDescriptor) {
        self.shapes.insert(shape_id, descriptor);
    }

    ///
    /// Adds an edge to this plan
    ///
    #[inline]
    pub fn add_edge(&mut self, new_edge: TEdge) {
        self.edges.push(new_edge);
    }

    ///
    /// Declares a shape and all of its edges at once
    ///
    pub fn add_shape(&mut self, shape_id: ShapeId, descriptor: ShapeDescriptor, edges: impl IntoIterator<Item=TEdge>) {
        self.declare_shape_description(shape_id, descriptor);
        for edge in edges {
            self.add_edge(edge);
        }
    }
}
