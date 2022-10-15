use super::drawing_request::*;
use crate::*;

use flo_scene::*;

use uuid::*;
use futures::prelude::*;
use futures::stream;

use std::time::{Duration};
use std::sync::*;

use std::collections::{HashMap};

// TODO: how to deal with things like updating the canvas transform/transforming all the sprites?
// TODO: might be better to work by taking drawing requests and sending them on? Or could have some commands that update the sprite layer's transform independently.

pub const SPRITE_LAYERS: EntityId = EntityId::well_known(uuid!["E025F272-D3E7-4417-BC51-51A22805167D"]);

///
/// Request for the sprite properties layer
///
#[derive(Clone, Debug)]
pub enum SpriteLayerRequest {
    /// Sets the transform applied to the sprites layer
    SetTransform(Option<Transform2D>),

    /// Changes the sprite layer, and re-renders the sprites onto that layer. The original layer is left unchanged.
    SetLayer(LayerId),

    /// Changes the base sprite ID (forcing a re-render of the sprites)
    SetBaseSpriteId(SpriteId),

    /// Changes the refresh rate (time between an invalidatation and a canvas update) of the sprite layer
    SetRefreshRate(Duration),
}

///
/// Represents the position of a sprite
///
#[derive(Clone, PartialEq, Debug)]
pub struct SpriteTransform {
    /// The transform for this sprite
    pub transform: Transform2D,

    /// Sprites with a lower z-index are drawn behind sprites with a higher z-index
    pub zindex: f64,
}

///
/// Request for the sprite properties layer, including messages that are only delivered internally
///
#[derive(Clone, Debug)]
enum InternalSpriteLayerRequest {
    /// Sets the transform applied to the sprites layer (or None to use whatever transform is left on the )
    SetTransform(Option<Transform2D>),

    /// Changes the sprite layer, and re-renders the sprites onto that layer. The original layer is left unchanged.
    SetLayer(LayerId),

    /// Changes the base sprite ID (forcing a re-render of the sprites)
    SetBaseSpriteId(SpriteId),

    /// Changes the refresh rate (time between an invalidatation and a canvas update) of the sprite layer
    SetRefreshRate(Duration),

    /// Sets the sprite definition for an entity
    SetSpriteDefinition(EntityId, Arc<Vec<Draw>>),

    /// Sets the sprite transforms for an entity (sprite is drawn at all of these positions)
    SetSpriteTransform(EntityId, Vec<SpriteTransform>),

    /// The sprite for an entity was removed
    DeleteSprite(EntityId),
}

impl From<SpriteLayerRequest> for InternalSpriteLayerRequest {
    fn from(msg: SpriteLayerRequest) -> InternalSpriteLayerRequest {
        match msg {
            SpriteLayerRequest::SetTransform(new_transform)     => { InternalSpriteLayerRequest::SetTransform(new_transform) }
            SpriteLayerRequest::SetLayer(new_layer)             => { InternalSpriteLayerRequest::SetLayer(new_layer) }
            SpriteLayerRequest::SetBaseSpriteId(new_sprite)     => { InternalSpriteLayerRequest::SetBaseSpriteId(new_sprite) }
            SpriteLayerRequest::SetRefreshRate(refresh_rate)    => { InternalSpriteLayerRequest::SetRefreshRate(refresh_rate) }
        }
    }
}

///
/// Creates a sprite layer entity in a scene
///
/// This will monitor the properties for all other entities in a scene for the `SpriteDefinition` property (of type `Arc<Vec<Draw>>`) and
/// `SpriteTransform` property (of type `Vec<SpriteTransform>`). The SpriteDefinition is used to define and update a sprite in the 
/// canvas, and the sprite is then drawn on the sprite layer at the position(s) specified by the sprite transform.
///
/// The sprite layer uses its own transform for rendering, which can be adjusted by sending commands to the sprite layer entity.
///
pub fn create_sprite_layer_entity(entity_id: EntityId, context: &Arc<SceneContext>, initial_sprite_id: SpriteId, sprite_layer: LayerId, canvas: impl 'static + EntityChannel<Message=DrawingRequest>) -> Result<impl EntityChannel<Message=SpriteLayerRequest>, CreateEntityError> {
    // Convert between the internal request and the external request type
    context.convert_message::<SpriteLayerRequest, InternalSpriteLayerRequest>()?;

    // Create the entity
    context.create_entity(entity_id, move |context, messages| async move {
        // Track sprite definitions and turn them into SetSpriteDefinition requests
        let sprite_definitions = properties_follow_all::<Arc<Vec<Draw>>>(&context, "SpriteDefinition").flat_map(|msg| {
            match msg {
                FollowAll::NewValue(entity_id, sprite_definition)   => stream::iter(Some(InternalSpriteLayerRequest::SetSpriteDefinition(entity_id, sprite_definition))),
                FollowAll::Destroyed(entity_id)                     => stream::iter(Some(InternalSpriteLayerRequest::DeleteSprite(entity_id))),
                FollowAll::Error(_)                                 => stream::iter(None),
            }
        });

        // Track sprite translations
        let sprite_transforms = properties_follow_all::<Vec<SpriteTransform>>(&context, "SpriteTransform").flat_map(|msg| {
            match msg {
                FollowAll::NewValue(entity_id, sprite_transform)    => stream::iter(Some(InternalSpriteLayerRequest::SetSpriteTransform(entity_id, sprite_transform))),
                FollowAll::Destroyed(entity_id)                     => stream::iter(None),
                FollowAll::Error(_)                                 => stream::iter(None),
            }
        });

        // Entity state variables
        let mut layer_transform         = None;
        let mut sprite_layer            = LayerId(1);
        let mut base_sprite_id          = 10000;
        let mut sprite_for_entity       = HashMap::new();
        let mut transforms_for_entity   = HashMap::new();
        let mut next_offset             = 0;
        let mut free_sprite_offsets     = vec![];
        let mut refresh_rate            = Duration::from_nanos(1_000_000_000 / 120);

        // Mix in the definition updates with the other messages
        let messages        = stream::select_all(vec![messages.boxed(), sprite_definitions.boxed(), sprite_transforms.boxed()]);
        let mut messages    = messages.ready_chunks(50);
        let mut canvas      = canvas;

        while let Some(msg_chunk) = messages.next().await {
            // Messages are read in chunks (so we don't redraw while messages are waiting)
            for msg in msg_chunk.into_iter() {
                use InternalSpriteLayerRequest::*;

                match msg {
                    SetTransform(new_transform)             => { layer_transform = new_transform; },        // TODO: trigger redraw
                    SetLayer(new_layer)                     => { sprite_layer = new_layer; },               // TODO: trigger redraw
                    SetBaseSpriteId(SpriteId(new_sprite))   => { base_sprite_id = new_sprite; },            // TODO: renumber sprites, 
                    SetRefreshRate(new_refresh_rate)        => { refresh_rate = new_refresh_rate },

                    SetSpriteDefinition(entity_id, drawing) => {
                        // Allocate a sprite ID
                        let sprite_offset = if let Some(allocated_offset) = sprite_for_entity.get(&entity_id) {
                            *allocated_offset
                        } else if let Some(offset) = free_sprite_offsets.pop() {
                            sprite_for_entity.insert(entity_id, offset);
                            offset
                        } else {
                            let offset  = next_offset;
                            sprite_for_entity.insert(entity_id, offset);
                            next_offset += 1;
                            offset
                        };

                        // Send the definition for this sprite so it's ready to draw
                        canvas.send(DrawingRequest::Draw(Arc::new(vec![
                            Draw::PushState,
                            Draw::Sprite(SpriteId(base_sprite_id + sprite_offset)),
                            Draw::ClearSprite,
                        ]))).await.ok();
                        canvas.send(DrawingRequest::Draw(Arc::clone(&drawing)));
                        canvas.send(DrawingRequest::Draw(Arc::new(vec![
                            Draw::PopState,
                        ]))).await.ok();

                        // TODO: trigger redraw
                    },

                    SetSpriteTransform(entity_id, transform) => {
                        let transform_changed = if let Some(old_transform) = transforms_for_entity.get(&entity_id) {
                            &transform == old_transform
                        } else {
                            true
                        };

                        // Update the transform for this sprite
                        transforms_for_entity.insert(entity_id, transform);

                        // TODO: trigger redraw
                        if transform_changed {

                        }
                    },

                    DeleteSprite(entity_id) => {
                        // Remove the transform and the sprite
                        let removed_sprite      = sprite_for_entity.remove(&entity_id).is_some();
                        let removed_transforms  = transforms_for_entity.remove(&entity_id).is_some();

                        // TODO: trigger redraw, if they existed
                        if removed_sprite || removed_transforms {

                        }
                    },
                }
            }
        }
    }).map(|channel| channel.convert_message())
}
