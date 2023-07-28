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

    ///
    /// Returns the edges that intercept a scanline. Shapes are entered on the right-hand side of any intercepts.
    ///
    pub fn intercepts_on_scanline<'a>(&'a self, y_pos: f64) -> impl 'a + Iterator<Item=(ShapeId, EdgeInterceptDirection, f64)> {
        // This is the slow way to find the edges that intercept a scanline
        // Possible enhancements
        //  - group up the edges by y position (we can use regions here) so that it's easy to find which edges are on a particular scanline
        //  - pre-sort the edges and only re-sort if there are overlapping edges. Most of the time in an edge region the edges will be intercepted in the
        //      same order
        //  - for anti-aliasing we need a way to track intercepts on the previous scanline for the same shape (usually the same edge, but sometimes the preceding or following edge)
        let mut intercepts = vec![];

        for edge in self.edges.iter() {
            for (direction, pos) in edge.intercepts(y_pos) {
                intercepts.push((edge.shape(), direction, pos));
            }
        }

        intercepts.sort_by(|(_, _, pos_a), (_, _, pos_b)| pos_a.total_cmp(pos_b));
        intercepts.into_iter()
    }
}
