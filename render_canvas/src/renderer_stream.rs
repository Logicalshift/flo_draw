use super::render_entity::*;
use super::renderer_core::*;

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

///
/// Represents the active state of the render stream
///
#[derive(Clone, Copy)]
struct RenderStreamState {
    /// The render target
    render_target: Option<render::RenderTargetId>,

    /// The blend mode to use
    blend_mode: Option<render::BlendMode>,

    /// The shader to use
    shader: Option<render::ShaderType>,

    /// The transform to apply to the rendering instructions
    transform: Option<canvas::Transform2D>
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

impl RenderStreamState {
    ///
    /// Creates a new render stream state
    ///
    fn new() -> RenderStreamState {
        RenderStreamState {
            render_target:  None,
            blend_mode:     None,
            shader:         None,
            transform:      None
        }
    }

    ///
    /// Returns the render actions needed to update from the specified state to this state (in reverse order, for replaying as a render stack)
    ///
    fn update_from_state(&self, from: &RenderStreamState) -> Vec<render::RenderAction> {
        let mut updates = vec![];

        if let Some(transform) = self.transform {
            if Some(transform) != from.transform {
                updates.push(render::RenderAction::SetTransform(transform_to_matrix(&transform)));
            }
        }

        if let Some(shader) = self.shader {
            if Some(shader) != from.shader {
                updates.push(render::RenderAction::UseShader(shader));
            }
        }

        if let Some(blend_mode) = self.blend_mode {
            if Some(blend_mode) != from.blend_mode {
                updates.push(render::RenderAction::BlendMode(blend_mode));
            }
        }

        if let Some(render_target) = self.render_target {
            if Some(render_target) != from.render_target {
                updates.push(render::RenderAction::SelectRenderTarget(render_target));
            }
        }

        updates
    }
}

impl RenderCore {
    ///
    /// Generates the rendering actions for the layer with the specified handle
    ///
    fn render_layer(&mut self, viewport_transform: canvas::Transform2D, layer_handle: LayerHandle) -> Vec<render::RenderAction> {
        use self::RenderEntity::*;

        let core = self;

        // Render the layer in reverse order (this is a stack, so operations are run in reverse order)
        let mut render_layer_stack  = vec![];
        let mut active_transform    = canvas::Transform2D::identity();
        let mut active_blend_mode   = render::BlendMode::DestinationOver;
        let mut use_erase_texture   = false;
        let mut layer               = core.layer(layer_handle);

        for render_idx in 0..layer.render_order.len() {
            match &layer.render_order[render_idx] {
                Missing => {
                    // Temporary state while sending a vertex buffer?
                    panic!("Tessellation is not complete (vertex buffer went missing)");
                },

                Tessellating(_id) => { 
                    // Being processed? (shouldn't happen)
                    panic!("Tessellation is not complete (tried to render too early)");
                },

                VertexBuffer(_buffers) => {
                    // Should already have sent all the vertex buffers
                    panic!("Tessellation is not complete (found unexpected vertex buffer in layer)");
                },

                RenderSprite(sprite_id, sprite_transform) => { 
                    let sprite_id           = *sprite_id;
                    let sprite_transform    = *sprite_transform;

                    if let Some(sprite_layer) = core.sprites.get(&sprite_id) {
                        let sprite_layer = *sprite_layer;

                        // Set the transform for the preceding rendering instructions
                        let combined_transform  = &viewport_transform * &active_transform;
                        let combined_matrix     = transform_to_matrix(&combined_transform);

                        render_layer_stack.push(render::RenderAction::SetTransform(combined_matrix));

                        if active_blend_mode == render::BlendMode::DestinationOut {
                            // Preceding renders need to update the erase texture
                            render_layer_stack.push(render::RenderAction::BlendMode(render::BlendMode::AllChannelAlphaDestinationOver));
                            render_layer_stack.push(render::RenderAction::UseShader(render::ShaderType::Simple { erase_texture: None }));
                            render_layer_stack.push(render::RenderAction::SelectRenderTarget(render::RenderTargetId(1)));
                        } else {
                            render_layer_stack.push(render::RenderAction::BlendMode(active_blend_mode));
                            render_layer_stack.push(render::RenderAction::UseShader(render::ShaderType::Simple { erase_texture: None }));
                            render_layer_stack.push(render::RenderAction::SelectRenderTarget(render::RenderTargetId(0)));
                        }

                        // The sprite transform is appended to the viewport transform
                        let sprite_transform = combined_transform * sprite_transform;

                        // Render the layer associated with the sprite
                        render_layer_stack.extend(core.render_layer(sprite_transform, sprite_layer));
                    }

                    // Reborrow the layer
                    layer                   = core.layer(layer_handle);
                },

                SetTransform(new_transform) => {
                    // The 'active transform' applies to all the preceding render instructions
                    let combined_transform  = &viewport_transform * &active_transform;
                    let combined_matrix     = transform_to_matrix(&combined_transform);

                    if render_layer_stack.len() > 0 {
                        render_layer_stack.push(render::RenderAction::SetTransform(combined_matrix));
                    }

                    // The new transform will apply to all the following render instructions
                    active_transform        = *new_transform;
                },

                SetBlendMode(new_blend_mode) => {
                    // Update the blend mode for the preceding render instructions
                    if active_blend_mode == render::BlendMode::DestinationOut {

                        let combined_transform  = &viewport_transform * &active_transform;
                        let combined_matrix     = transform_to_matrix(&combined_transform);

                        render_layer_stack.push(render::RenderAction::SetTransform(combined_matrix));

                        // Flag that we're using the erase texture and it needs to be cleared for this layer
                        use_erase_texture       = true;

                        // Preceding renders need to update the erase texture
                        render_layer_stack.push(render::RenderAction::SelectRenderTarget(render::RenderTargetId(1)));
                        render_layer_stack.push(render::RenderAction::UseShader(render::ShaderType::Simple { erase_texture: None }));
                        render_layer_stack.push(render::RenderAction::BlendMode(render::BlendMode::AllChannelAlphaDestinationOver));

                    } else {

                        if new_blend_mode == &render::BlendMode::DestinationOut {
                            let combined_transform  = &viewport_transform * &active_transform;
                            let combined_matrix     = transform_to_matrix(&combined_transform);

                            render_layer_stack.push(render::RenderAction::SetTransform(combined_matrix));

                            // Need to update the blend mode to disable the eraser, and apply the eraser texture to the preceding renders
                            render_layer_stack.push(render::RenderAction::UseShader(render::ShaderType::Simple { erase_texture: Some(render::TextureId(1)) }));
                        }

                        // Preceding renders need to use the specified blend mode
                        render_layer_stack.push(render::RenderAction::SelectRenderTarget(render::RenderTargetId(0)));
                        render_layer_stack.push(render::RenderAction::BlendMode(active_blend_mode));

                    }

                    // active_blend_mode will eventually be applied to the following instructions
                    active_blend_mode = *new_blend_mode;
                },

                DrawIndexed(vertex_buffer, index_buffer, num_items) => {
                    // Draw the triangles
                    render_layer_stack.push(render::RenderAction::DrawIndexedTriangles(*vertex_buffer, *index_buffer, *num_items));
                }
            }
        }

        // The 'active transform' applies to all the preceding render instructions
        let combined_transform  = &viewport_transform * &active_transform;
        let combined_matrix     = transform_to_matrix(&combined_transform);

        render_layer_stack.push(render::RenderAction::SetTransform(combined_matrix));

        if active_blend_mode == render::BlendMode::DestinationOut {

            // Preceding renders need to update the erase texture
            render_layer_stack.push(render::RenderAction::BlendMode(render::BlendMode::AllChannelAlphaDestinationOver));
            render_layer_stack.push(render::RenderAction::UseShader(render::ShaderType::Simple { erase_texture: None }));
            render_layer_stack.push(render::RenderAction::SelectRenderTarget(render::RenderTargetId(1)));

        } else {
            render_layer_stack.push(render::RenderAction::BlendMode(active_blend_mode));
            render_layer_stack.push(render::RenderAction::UseShader(render::ShaderType::Simple { erase_texture: None }));
            render_layer_stack.push(render::RenderAction::SelectRenderTarget(render::RenderTargetId(0)));
        }

        // Clear the erase mask if it's used on this layer
        if use_erase_texture {
            render_layer_stack.push(render::RenderAction::Clear(render::Rgba8([0, 0, 0, 0])));
            render_layer_stack.push(render::RenderAction::SelectRenderTarget(render::RenderTargetId(1)));
        }

        // Generate a pending set of actions for the current layer
        return render_layer_stack;
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
                // Layers are rendered in reverse order
                self.processing_future  = None;
                self.layer_id           = self.core.sync(|core| core.layers.len());
                self.render_index       = 0;
            }

        }

        // We've generated all the vertex buffers: generate the instructions to render them
        let mut layer_id        = self.layer_id;
        let viewport_transform  = self.viewport_transform;

        let result              = if layer_id == 0 {
            // Stop if we've processed all the layers
            None
        } else {
            // Move to the previous layer
            layer_id -= 1;

            self.core.sync(|core| {
                // Send any pending vertex buffers, then render the layer (note that the rendering is a stack, so the vertex buffers go on the end)
                let layer_handle        = core.layers[layer_id];
                let send_vertex_buffers = core.send_vertex_buffers(layer_handle);

                let mut render_layer    = core.render_layer(viewport_transform, layer_handle);
                render_layer.extend(send_vertex_buffers);

                Some(render_layer)
            })
        };

        // Update the layer ID to continue iterating
        self.layer_id       = layer_id;

        // Add the result to the pending queue
        if let Some(result) = result {
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
