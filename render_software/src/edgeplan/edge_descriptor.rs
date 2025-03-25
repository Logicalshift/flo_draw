use super::edge_descriptor_intercept::*;
use super::shape_id::*;

use flo_canvas as canvas;

use std::sync::*;

///
/// Describes the edge of a shape. Shapes are described by determining where a ray cast along the x axis intercepts their edges.
///
/// Edge descriptors are used as part of an edge plan to describe a scene. The shape ID is used to apply attributes to the shape.
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
    fn intercepts(&self, y_positions: &[f64], output: &mut [Vec<EdgeDescriptorIntercept>]);

    ///
    /// Finds the apexes for this shape and appends them to the output Vec
    ///
    /// These are the points along the y-axis for this shape where the direction the edge is moving in changes
    /// direction (from moving up to moving down or vice-versa). These can be used to multi-sample lines where
    /// a shape only partially overlaps it, which can reduce the number of artifacts that appear for intricate
    /// shapes with small lines - particularly for cases where there's a horizontal line with sub-pixel,
    /// or small shapes where single-pixel inaccuracies are noticeable.
    ///
    /// The output can be left empty if this shape does not support calculating apexes.
    ///
    fn apexes(&self, output: &mut Vec<f64>);

    ///
    /// The number of samples to take when processing a section of this edge that has high detail
    ///
    /// The apexes are used to determine where an edge has detail that's finer than a single pixel in the
    /// vertical axis. To render these sections more accurately, one approach is to super-sample the region.
    /// When we do this for this edge, we'll use the specified number of samples. Note that it's important
    /// that all edges making up a single shape use the same number of samples.
    ///
    fn detail_samples(&self) -> usize { 5 }

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
    #[inline] fn description(&self) -> String                       { (**self).description() }
    #[inline] fn apexes(&self, output: &mut Vec<f64>)               { (**self).apexes(output) }
    #[inline] fn intercepts(&self, y_positions: &[f64], output: &mut [Vec<EdgeDescriptorIntercept>]) { 
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
    #[inline] fn description(&self) -> String                       { (**self).description() }

    #[inline] fn apexes(&self, output: &mut Vec<f64>)               { (**self).apexes(output) }

    #[inline] fn intercepts(&self, y_positions: &[f64], output: &mut [Vec<EdgeDescriptorIntercept>]) { 
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
