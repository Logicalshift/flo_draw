use super::edge_descriptor::*;
use super::shape_descriptor::*;
use super::shape_id::*;

use smallvec::*;

use flo_sparse_array::*;

///
/// An edge plan describes a 2 dimensional space as a set of edges that divide 
///
pub struct EdgePlan<TEdge>
where
    TEdge: EdgeDescriptor,
{
    /// Describes the shapes
    shapes: SparseArray<ShapeDescriptor>,

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
            shapes: SparseArray::empty(),
            edges:  vec![],
        }
    }

    ///
    /// Stores the details of how the interior of a shape should be rendered
    ///
    pub fn declare_shape_description(&mut self, shape_id: ShapeId, descriptor: ShapeDescriptor) {
        self.shapes.insert(shape_id.0, descriptor);
    }

    ///
    /// Returns the z-index for a shape ID
    ///
    #[inline]
    pub fn shape_z_index(&self, shape_id: ShapeId) -> i64 {
        self.shapes.get(shape_id.0).map(|shape| shape.z_index).unwrap_or(0)
    }

    ///
    /// Returns the shape descriptor for a shape ID
    ///
    #[inline]
    pub fn shape_descriptor(&self, shape_id: ShapeId) -> Option<&ShapeDescriptor> {
        self.shapes.get(shape_id.0)
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
    pub fn intercepts_on_scanlines<'a>(&'a self, y_positions: &[f64], output: &mut [SmallVec<[(ShapeId, EdgeInterceptDirection, f64); 4]>]) {
        // Extend the edge intercepts to cover the number of y-positions we have (can be larger than needed but not smaller)
        let mut edge_intercepts = vec![smallvec![]; y_positions.len()];

        // Clear the output
        output.iter_mut().for_each(|val| val.clear());

        // This is the slow way to find the edges that intercept a scanline
        // Possible enhancements
        //  - group up the edges by y position (we can use regions here) so that it's easy to find which edges are on a particular scanline
        //  - pre-sort the edges and only re-sort if there are overlapping edges. Most of the time in an edge region the edges will be intercepted in the
        //      same order
        //  - for anti-aliasing we need a way to track intercepts on the previous scanline for the same shape (usually the same edge, but sometimes the preceding or following edge)
        for edge in self.edges.iter() {
            // Read the intercepts from this edge (we rely on the 'intercepts' method overwriting any old values)
            edge.intercepts(y_positions, &mut edge_intercepts);

            for idx in 0..y_positions.len() {
                let output = &mut output[idx];

                for (direction, pos) in edge_intercepts[idx] {
                    output.push((edge.shape(), direction, pos));
                }
            }
        }

        // Sort the intercepts on each line by x position
        output.iter_mut().for_each(|intercepts| {
            intercepts.sort_by(|(_, _, pos_a), (_, _, pos_b)| pos_a.total_cmp(pos_b));
        });
    }
}
