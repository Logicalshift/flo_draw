use super::edge_descriptor_intercept::*;
use super::shape_id::*;

use flo_canvas as canvas;
use smallvec::*;

use std::sync::*;

///
/// Describes an edge that can be used as part of an edge plan
///
pub trait EdgeDescriptor : Send + Sync {
    ///
    /// Creates a clone of this edge as an Arc<dyn EdgeDescriptor>
    ///
    fn clone_as_object(&self) -> Arc<dyn EdgeDescriptor>;

    ///
    /// Performs any pre-calculations needed before the `intercepts()` call can be made
    ///
    /// This serves two purposes: firstly if an edge is never actually used for rendering, this will never be called,
    /// speeding up how a scene is rendered. Secondly this can be run in parallel for all of the edges in a scene,
    /// increasing the performance on multi-core systems.
    ///
    fn prepare_to_render(&mut self);

    ///
    /// Returns a transformed version of this edge descriptor
    ///
    /// This both applies the transform and prepares the result for rendering.
    ///
    fn transform(&self, transform: &canvas::Transform2D) -> Arc<dyn EdgeDescriptor>;

    ///
    /// Returns the ID of the shape that this edge is a boundary for
    ///
    /// Edges represent the boundary between the region outside of this shape and the region inside of it. Shapes may
    /// have additional data associated with them, such as a set of programs to run to generate pixels on the inside
    /// and a z-index to indicate which order to draw the shapes in.
    ///
    fn shape(&self) -> ShapeId;

    ///
    /// The minimum and maximum coordinates where this edge might be found
    ///
    /// This does not have to be 100% accurate, so long as the edge is entirely contained within the bounds
    ///
    fn bounding_box(&self) -> ((f64, f64), (f64, f64));

    ///
    /// Appends the intercepts for this edge at a set of y positions to a buffer
    ///
    /// `prepare_to_render()` must have been called on this edge at least once before before this is called.
    /// This function may not return valid results until this has been done.
    ///
    /// The API here returns intercepts for as many y-positions as needed: this is more efficient with the
    /// layered design of this renderer as it makes it possible to run the inner loop of the algorithm
    /// multiple times (or even take advantage of vectorisation), and use previous results to derive future
    /// results. The output list should be as long as the y-positions list and will be entirely overwritten 
    /// when this returns.
    ///
    /// As a general convention, end points should not be included in edges, as there should be an attached 
    /// edge with a start point at the same position. Apex points, where the following edge moves away in 
    /// the y-axis, also should not be counted as an intercept.
    ///
    fn intercepts(&self, y_positions: &[f64], output: &mut [SmallVec<[EdgeDescriptorIntercept; 2]>]);

    ///
    /// For debugging, an optional description of this edge
    ///
    fn description(&self) -> String { "no description".to_string() }
}

impl EdgeDescriptor for Box<dyn EdgeDescriptor> {
    #[inline] fn clone_as_object(&self) -> Arc<dyn EdgeDescriptor>  { (**self).clone_as_object() }
    #[inline] fn prepare_to_render(&mut self)                       { (**self).prepare_to_render() }
    #[inline] fn shape(&self) -> ShapeId                            { (**self).shape() }
    #[inline] fn bounding_box(&self) -> ((f64, f64), (f64, f64))    { (**self).bounding_box() }
    #[inline] fn intercepts(&self, y_positions: &[f64], output: &mut [SmallVec<[EdgeDescriptorIntercept; 2]>]) { 
        (**self).intercepts(y_positions, output) 
    }
    #[inline] fn transform(&self, transform: &canvas::Transform2D) -> Arc<dyn EdgeDescriptor> {
        (**self).transform(transform)
    }
}

impl EdgeDescriptor for Arc<dyn EdgeDescriptor> {
    #[inline] fn clone_as_object(&self) -> Arc<dyn EdgeDescriptor>  { (**self).clone_as_object() }
    #[inline] fn shape(&self) -> ShapeId                            { (**self).shape() }
    #[inline] fn bounding_box(&self) -> ((f64, f64), (f64, f64))    { (**self).bounding_box() }

    #[inline] fn intercepts(&self, y_positions: &[f64], output: &mut [SmallVec<[EdgeDescriptorIntercept; 2]>]) { 
        (**self).intercepts(y_positions, output) 
    }

    #[inline] fn transform(&self, transform: &canvas::Transform2D) -> Arc<dyn EdgeDescriptor> {
        (**self).transform(transform)
    }

    #[inline] fn prepare_to_render(&mut self) { 
        if let Some(inner) = Arc::get_mut(self) {
            // This is the only copy of this object, so we can mutate it
            inner.prepare_to_render();
        } else {
            // Clone as a new object
            *self = (**self).clone_as_object();

            // Must be the only copy, so we can retrieve it as mutable and then call prepare_to_render
            let inner = Arc::get_mut(self).unwrap();
            inner.prepare_to_render();
        }
    }
}
