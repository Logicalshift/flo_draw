use super::render_entity::*;
use super::renderer_core::*;

use flo_canvas as canvas;
use flo_render as render;

use ::desync::*;

use futures::prelude::*;
use futures::task::{Context, Poll};
use futures::future::{BoxFuture};

use std::pin::*;
use std::sync::*;

///
/// Tri-state version of 'option' that supports 'Unknown' as well as None and Some
///
#[derive(Clone, Copy, PartialEq)]
enum Maybe<T> {
    Unknown,
    None,
    Some(T)
}

///
/// Stream of rendering actions resulting from a draw instruction
///
pub struct RenderStream<'a> {
    /// The core where the render instructions are read from
    core: Arc<Desync<RenderCore>>,

    /// The ID of the buffer to use for rendering the background quad
    background_vertex_buffer: render::VertexBufferId,

    /// True if the frame is suspended (we're not going to generate any direct rendering due to this drawing operation)
    frame_suspended: bool,

    /// The future that is processing new drawing instructions
    processing_future: Option<BoxFuture<'a, ()>>,

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

    /// The texture to use as the eraser mask (None for no eraser texture)
    erase_mask: Maybe<render::TextureId>,

    /// The texture to use for the clip mask (None for no clip mask)
    clip_mask: Maybe<render::TextureId>,

    /// The transform to apply to the rendering instructions
    transform: Option<canvas::Transform2D>
}

impl<'a> RenderStream<'a> {
    ///
    /// Creates a new render stream
    ///
    pub fn new<ProcessFuture>(core: Arc<Desync<RenderCore>>, frame_suspended: bool, processing_future: ProcessFuture, viewport_transform: canvas::Transform2D, background_vertex_buffer: render::VertexBufferId, initial_action_stack: Vec<render::RenderAction>, final_action_stack: Vec<render::RenderAction>) -> RenderStream<'a>
    where   ProcessFuture: 'a+Send+Future<Output=()> {
        RenderStream {
            core:                       core,
            frame_suspended:            frame_suspended,
            background_vertex_buffer:   background_vertex_buffer,
            processing_future:          Some(processing_future.boxed()),
            pending_stack:              initial_action_stack,
            final_stack:                Some(final_action_stack),
            viewport_transform:         viewport_transform,
            layer_id:                   0,
            render_index:               0
        }
    }
}

impl<T> Maybe<T> {
    ///
    /// Converts to an optional value
    ///
    pub fn value(self) -> Option<Option<T>> {
        match self {
            Maybe::Unknown      => None,
            Maybe::None         => Some(None),
            Maybe::Some(val)    => Some(Some(val))
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
            erase_mask:     Maybe::Unknown,
            clip_mask:      Maybe::Unknown, 
            transform:      None
        }
    }

    ///
    /// Returns the render actions needed to update from the specified state to this state (in reverse order, for replaying as a render stack)
    ///
    fn update_from_state(&self, from: &RenderStreamState) -> Vec<render::RenderAction> {
        let mut updates = vec![];

        if let Some(transform) = self.transform {
            if Some(transform) != from.transform || (self.render_target != from.render_target && self.render_target.is_some()) {
                updates.push(render::RenderAction::SetTransform(transform_to_matrix(&transform)));
            }
        }

        if let (Some(erase), Some(clip)) = (self.erase_mask.value(), self.clip_mask.value()) {
            let mask_textures_changed = Some(erase) != from.erase_mask.value() || Some(clip) != from.clip_mask.value();
            let render_target_changed = self.render_target != from.render_target && self.render_target.is_some();

            if mask_textures_changed || render_target_changed {
                let shader = render::ShaderType::Simple { erase_texture: erase, clip_texture: clip };
                updates.push(render::RenderAction::UseShader(shader));
            }
        }

        if let Some(blend_mode) = self.blend_mode {
            if Some(blend_mode) != from.blend_mode || (self.render_target != from.render_target && self.render_target.is_some()) {
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
    /// The render state passed in is the expected state after this rendering has completed, and is updated to be the expected state
    /// before the rendering is completed. This slightly weird arrangement is because the rendering operations are returned as a stack:
    /// ie, they'll run in reverse order.
    ///
    fn render_layer(&mut self, viewport_transform: canvas::Transform2D, layer_handle: LayerHandle, render_state: &mut RenderStreamState) -> Vec<render::RenderAction> {
        use self::RenderEntity::*;

        let core = self;

        // Render the layer in reverse order (this is a stack, so operations are run in reverse order)
        let mut render_layer_stack  = vec![];
        let mut active_transform    = canvas::Transform2D::identity();
        let mut use_erase_texture   = false;
        let mut layer               = core.layer(layer_handle);

        render_state.transform      = Some(viewport_transform);
        render_state.blend_mode     = Some(render::BlendMode::DestinationOver);
        render_state.render_target  = Some(render::RenderTargetId(0));
        render_state.erase_mask     = Maybe::None;
        render_state.clip_mask      = Maybe::None;

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

                        // The sprite transform is appended to the viewport transform
                        let combined_transform  = &viewport_transform * &active_transform;
                        let sprite_transform    = combined_transform * sprite_transform;

                        // The items from before the sprite should be rendered using the current state
                        let old_state           = *render_state;

                        // Render the layer associated with the sprite
                        let render_sprite       = core.render_layer(sprite_transform, sprite_layer, render_state);

                        // Items before the sprite are rendered using the 'pre-sprite' rendering
                        render_layer_stack.extend(old_state.update_from_state(render_state));

                        // ... before that, the sprite is renderered
                        render_layer_stack.extend(render_sprite);

                        // ... using its render state
                        render_layer_stack.extend(render_state.update_from_state(&old_state));

                        // Following instructions are rendered using the state before the sprite
                        *render_state           = old_state;
                    }

                    // Reborrow the layer
                    layer                   = core.layer(layer_handle);
                },

                SetTransform(new_transform) => {
                    // The new transform will apply to all the following render instructions
                    active_transform        = *new_transform;

                    // The preceding instructions should render according to the previous state
                    let old_state           = *render_state;
                    render_state.transform  = Some(&viewport_transform * &active_transform);

                    render_layer_stack.extend(old_state.update_from_state(render_state));
                },

                SetBlendMode(new_blend_mode) => {
                    let mut old_state   = *render_state;

                    if new_blend_mode == &render::BlendMode::DestinationOut {
                        // The previous state should use the eraser texture that we're abount to generate
                        if old_state.render_target == Some(render::RenderTargetId(0)) {
                            old_state.erase_mask = Maybe::Some(render::TextureId(1));
                        }

                        // Render to the eraser texture
                        render_state.blend_mode     = Some(render::BlendMode::AllChannelAlphaDestinationOver);
                        render_state.render_target  = Some(render::RenderTargetId(1));
                        render_state.erase_mask     = Maybe::None;

                        // Flag that we're using the erase texture and it needs to be cleared for this layer
                        use_erase_texture       = true;
                    } else {
                        // Render the main buffer
                        render_state.blend_mode     = Some(*new_blend_mode);
                        render_state.render_target  = Some(render::RenderTargetId(0));

                        // Use the eraser texture if one is specified
                        if use_erase_texture {
                            render_state.erase_mask = Maybe::Some(render::TextureId(1));
                        } else {
                            render_state.erase_mask = Maybe::None;
                        }
                    }

                    // Apply the old state for the preceding instrucitons
                    render_layer_stack.extend(old_state.update_from_state(render_state));
                },

                DrawIndexed(vertex_buffer, index_buffer, num_items) => {
                    // Draw the triangles
                    render_layer_stack.push(render::RenderAction::DrawIndexedTriangles(*vertex_buffer, *index_buffer, *num_items));
                }
            }
        }

        // Clear the erase mask if it's used on this layer
        if use_erase_texture {
            render_state.render_target.map(|render_target| {
                render_layer_stack.push(render::RenderAction::SelectRenderTarget(render_target));
            });

            render_layer_stack.push(render::RenderAction::Clear(render::Rgba8([0, 0, 0, 0])));
            render_layer_stack.push(render::RenderAction::SelectRenderTarget(render::RenderTargetId(1)));
        }

        // Generate a pending set of actions for the current layer
        return render_layer_stack;
    }
}

impl<'a> RenderStream<'a> {
    ///
    /// Adds the instructions required to render the background colour to the pending queue
    ///
    fn render_background(&mut self) {
        let background_color = self.core.sync(|core| core.background_color);

        // If there's a background colour, then the finalize step should draw it (the OpenGL renderer has issues blitting alpha blended multisampled textures, so this hides that the 'clear' step above doesn't work there)
        let render::Rgba8([br, bg, bb, ba]) = background_color;

        if ba > 0 {
            let background_color = [br, bg, bb, ba];

            self.pending_stack.extend(vec![
                render::RenderAction::DrawTriangles(self.background_vertex_buffer, 0..6),
                render::RenderAction::UseShader(render::ShaderType::Simple { erase_texture: None, clip_texture: None }),
                render::RenderAction::BlendMode(render::BlendMode::DestinationOver),
                render::RenderAction::SetTransform(render::Matrix::identity()),

                // Generate a full-screen quad
                render::RenderAction::CreateVertex2DBuffer(self.background_vertex_buffer, vec![
                    render::Vertex2D { pos: [-1.0, -1.0],   tex_coord: [0.0, 0.0], color: background_color },
                    render::Vertex2D { pos: [1.0, 1.0],     tex_coord: [0.0, 0.0], color: background_color },
                    render::Vertex2D { pos: [1.0, -1.0],    tex_coord: [0.0, 0.0], color: background_color },

                    render::Vertex2D { pos: [-1.0, -1.0],   tex_coord: [0.0, 0.0], color: background_color },
                    render::Vertex2D { pos: [1.0, 1.0],     tex_coord: [0.0, 0.0], color: background_color },
                    render::Vertex2D { pos: [-1.0, 1.0],    tex_coord: [0.0, 0.0], color: background_color },
                ])
            ]);
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
                // Layers are rendered in reverse order
                self.processing_future  = None;
                self.layer_id           = self.core.sync(|core| core.layers.len());
                self.render_index       = 0;
            }

        }

        // We've generated all the vertex buffers: if frame rendering is suspended, stop here
        if self.frame_suspended {
            if let Some(final_actions) = self.final_stack.take() {
                self.pending_stack = final_actions;
                return Poll::Ready(self.pending_stack.pop());
            } else {
                return Poll::Ready(None);
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
                let mut render_state    = RenderStreamState::new();

                let mut render_layer    = core.render_layer(viewport_transform, layer_handle, &mut render_state);
                render_layer.extend(render_state.update_from_state(&RenderStreamState::new()));
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
            self.render_background();
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
