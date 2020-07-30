use super::renderer_core::*;
use super::renderer_layer::*;

use flo_canvas as canvas;
use flo_render as render;

use ::desync::*;

use futures::prelude::*;
use futures::task::{Context, Poll};
use futures::future::{LocalBoxFuture};

use std::pin::*;
use std::sync::*;

///
/// Stream of rendering actions resulting from a draw instruction
///
pub struct RenderStream<'a> {
    /// The core where the render instructions are read from
    core: Arc<Desync<RenderCore>>,

    /// The future that is processing new drawing instructions
    processing_future: Option<LocalBoxFuture<'a, ()>>,

    /// The current layer ID that we're processing
    layer_id: usize,

    /// The render entity within the layer that we're processing
    render_index: usize,

    /// Render actions waiting to be sent
    pending_stack: Vec<render::RenderAction>,

    /// The stack of operations to run when the rendering is complete (None if they've already been rendered)
    final_stack: Option<Vec<render::RenderAction>>,

    /// The transformation for the viewport
    viewport_transform: canvas::Transform2D
}

impl<'a> RenderStream<'a> {
    ///
    /// Creates a new render stream
    ///
    pub fn new<ProcessFuture>(core: Arc<Desync<RenderCore>>, processing_future: ProcessFuture, viewport_transform: canvas::Transform2D, initial_action_stack: Vec<render::RenderAction>, final_action_stack: Vec<render::RenderAction>) -> RenderStream<'a>
    where   ProcessFuture: 'a+Future<Output=()> {
        RenderStream {
            core:               core,
            processing_future:  Some(processing_future.boxed_local()),
            pending_stack:      initial_action_stack,
            final_stack:        Some(final_action_stack),
            viewport_transform: viewport_transform,
            layer_id:           0,
            render_index:       0
        }
    }
}

impl<'a> Stream for RenderStream<'a> {
    type Item = render::RenderAction;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<render::RenderAction>> { 
        // Return the next pending action if there is one
        if self.pending_stack.len() > 0 {
            // Note that pending is a stack, so the items are returned in reverse
            return Poll::Ready(self.pending_stack.pop());
        }

        // Poll the tessellation process if it's still running
        if let Some(processing_future) = self.processing_future.as_mut() {
            // Poll the future and send over any vertex buffers that might be waiting
            if processing_future.poll_unpin(context) == Poll::Pending {
                // Still generating render buffers
                // TODO: can potentially send the buffers to the renderer when they're generated here
                return Poll::Pending;
            } else {
                // Finished processing the rendering: can send the actual rendering commands to the hardware layer
                self.processing_future  = None;
                self.layer_id           = 0;
                self.render_index       = 0;
            }

        }

        // We've generated all the vertex buffers: generate the instructions to render them
        let mut layer_id        = self.layer_id;
        let mut render_index    = self.render_index;
        let viewport_transform  = self.viewport_transform;

        let result = self.core.sync(|core| {
            loop {
                if layer_id >= core.layers.len() {
                    // Reached the end of the layers
                    return vec![];
                }

                if render_index >= core.layers[layer_id].render_order.len() {
                    // Reached the end of the current layer
                    layer_id        += 1;
                    render_index    = 0;
                } else {
                    // layer_id, render_index is valid
                    break;
                }
            }

            // Action depends on the contents of the current render item
            use self::RenderEntity::*;
            match &core.layers[layer_id].render_order[render_index] {
                Missing => {
                    // Temporary state while sending a vertex buffer?
                    panic!("Tessellation is not complete (vertex buffer went missing)");
                },

                Tessellating(_op, _id) => { 
                    // Being processed? (shouldn't happen)
                    panic!("Tessellation is not complete (tried to render too early)");
                },

                SetTransform(new_transform) => {
                    // Move on to the next item
                    render_index            += 1;

                    // Set a new transformation
                    let combined_transform  = &viewport_transform * new_transform;
                    let combined_matrix     = transform_to_matrix(&combined_transform);

                    vec![render::RenderAction::SetTransform(combined_matrix)]
                },

                VertexBuffer(_op, _buffers) => {
                    // Ask the core to send this buffer for processing
                    core.send_vertex_buffer(layer_id, render_index)
                },

                DrawIndexed(_op, vertex_buffer, index_buffer, num_items) => {
                    // Move on to the next item to render
                    render_index += 1;

                    // Draw the triangles
                    vec![render::RenderAction::DrawIndexedTriangles(*vertex_buffer, *index_buffer, *num_items)]
                }
            }
        });

        // Update the layer and render index to continue iterating
        self.layer_id       = layer_id;
        self.render_index   = render_index;

        // Add the result to the pending queue
        if result.len() > 0 {
            // There are more actions to add to the pending stack
            self.pending_stack = result;
            return Poll::Ready(self.pending_stack.pop());
        } else if let Some(final_actions) = self.final_stack.take() {
            // There are no more drawing actions, but we have a set of final post-render instructions to execute
            self.pending_stack = final_actions;
            return Poll::Ready(self.pending_stack.pop());
        } else {
            // No further actions if the result was empty
            return Poll::Ready(None);
        }
    }
}

///
/// Converts a canvas transform to a rendering matrix
///
pub fn transform_to_matrix(transform: &canvas::Transform2D) -> render::Matrix {
    let canvas::Transform2D(t) = transform;

    render::Matrix([
        [t[0][0], t[0][1], 0.0, t[0][2]],
        [t[1][0], t[1][1], 0.0, t[1][2]],
        [t[2][0], t[2][1], 1.0, t[2][2]],
        [0.0,     0.0,     0.0, 1.0]
    ])
}
