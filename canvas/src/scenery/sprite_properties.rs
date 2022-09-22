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
    SetTransform(Transform2D),

    /// Changes the sprite layer, and re-renders the sprites onto that layer. The original layer is left unchanged.
    SetLayer(LayerId),

    /// Changes the base sprite ID (forcing a re-render of the sprites)
    SetBaseSpriteId(SpriteId),

    /// Changes the refresh rate (time between an invalidatation and a canvas update) of the sprite layer
    SetRefreshRate(Duration),
}

///
/// Request for the sprite properties layer, including messages that are only delivered internally
///
#[derive(Clone, Debug)]
enum InternalSpriteLayerRequest {
    /// Sets the transform applied to the sprites layer
    SetTransform(Transform2D),

    /// Changes the sprite layer, and re-renders the sprites onto that layer. The original layer is left unchanged.
    SetLayer(LayerId),

    /// Changes the base sprite ID (forcing a re-render of the sprites)
    SetBaseSpriteId(SpriteId),

    /// Changes the refresh rate (time between an invalidatation and a canvas update) of the sprite layer
    SetRefreshRate(Duration),

    /// Sets the sprite definition for an entity
    SetSpriteDefinition(EntityId, Arc<Vec<Draw>>),

    /// Sets the sprite transforms for an entity (sprite is drawn at all of these positions)
    SetSpriteTransform(EntityId, Vec<Transform2D>),

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
/// Optionally the property `SpriteZIndex`, of type `f64` can be used for sorting purposes.
///
/// The sprite layer uses its own transform for rendering, which can be adjusted by sending commands to the sprite layer entity.
///
pub fn create_sprite_layer_entity(entity_id: EntityId, context: &Arc<SceneContext>, initial_sprite_id: SpriteId, sprite_layer: LayerId, canvas: impl EntityChannel<Message=DrawingRequest>) -> Result<impl EntityChannel<Message=SpriteLayerRequest>, CreateEntityError> {
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
        let sprite_transforms = properties_follow_all::<Vec<Transform2D>>(&context, "SpriteTransform").flat_map(|msg| {
            match msg {
                FollowAll::NewValue(entity_id, sprite_transform)    => stream::iter(Some(InternalSpriteLayerRequest::SetSpriteTransform(entity_id, sprite_transform))),
                FollowAll::Destroyed(entity_id)                     => stream::iter(None),
                FollowAll::Error(_)                                 => stream::iter(None),
            }
        });

        // Entity state variables
        let mut layer_transform         = Transform2D::identity();
        let mut sprite_layer            = LayerId(1);
        let mut base_sprite_id          = 10000;
        //let mut sprite_for_entity       = HashMap::new();
        //let mut transforms_for_entity   = HashMap::new();
        //let mut free_sprite_offsets     = vec![];
        let mut refresh_rate            = Duration::from_nanos(1_000_000_000 / 120);

        // Mix in the definition updates with the other messages
        let messages        = stream::select_all(vec![messages.boxed(), sprite_definitions.boxed(), sprite_transforms.boxed()]);
        let mut messages    = messages.ready_chunks(50);

        while let Some(msg_chunk) = messages.next().await {
            for msg in msg_chunk.into_iter() {
                use InternalSpriteLayerRequest::*;

                match msg {
                    SetTransform(new_transform)             => { layer_transform = new_transform; },        // TODO: trigger redraw
                    SetLayer(new_layer)                     => { sprite_layer = new_layer; },               // TODO: trigger redraw
                    SetBaseSpriteId(SpriteId(new_sprite))   => { base_sprite_id = new_sprite; },            // TODO: renumber sprites, 
                    SetRefreshRate(new_refresh_rate)        => { refresh_rate = new_refresh_rate },

                    SetSpriteDefinition(entity_id, drawing)     => { todo!() },                         // TODO: allocate sprite ID, send sprite definition to rendering
                    SetSpriteTransform(entity_id, transform)    => { todo!() },                         // TODO: update transform, trigger redraw
                    DeleteSprite(entity_id)                     => { todo!() },                         // TODO: delete sprites/transforms for entity, trigger redraw
                }
            }
        }
    }).map(|channel| channel.convert_message())
}
