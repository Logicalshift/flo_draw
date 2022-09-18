use super::drawing_request::*;
use crate::*;

use flo_scene::*;

use uuid::*;
use futures::prelude::*;

use std::time::{Duration};
use std::sync::*;

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
/// This will monitor the properties for all other entities in a scene for the `SpriteDefinition` property (of type `Vec<Draw>`) and
/// `SpriteTransform` property (of type `Vec<SpriteTransform>`). The SpriteDefinition is used to define and update a sprite in the 
/// canvas, and the sprite is then drawn on the sprite layer at the position(s) specified by the sprite transform.
///
/// Optionally the property `SpriteZIndex`, of type `f64` can be used for sorting purposes.
///
/// The sprite layer uses its own transform for rendering, which can be adjusted by sending commands to the sprite layer entity.
///
pub fn create_sprite_layer_entity(entity_id: EntityId, context: &Arc<SceneContext>, initial_sprite_id: SpriteId, sprite_layer: LayerId, canvas: impl EntityChannel<Message=DrawingRequest>) -> Result<impl EntityChannel<Message=SpriteLayerRequest>, CreateEntityError> {
    // Convert between the internal request and the external request type
    context.convert_message::<SpriteLayerRequest, InternalSpriteLayerRequest>();

    // Create the entity
    context.create_entity(entity_id, move |context, messages| async move {
        // Fetch the properties channels
        let mut drawing_properties      = properties_channel::<Vec<Draw>>(PROPERTIES, &context).await.unwrap();
        let mut transform_properties    = properties_channel::<Vec<SpriteTransform>>(PROPERTIES, &context).await.unwrap();

        // Track the entities declaring particular properties
        let (sprite_definitions_sender, sprite_definitions) = SimpleEntityChannel::new(entity_id, 10);
        let (sprite_transforms_sender, sprite_transforms)   = SimpleEntityChannel::new(entity_id, 10);

        let sprite_definitions  = drawing_properties.send(PropertyRequest::TrackPropertiesWithName(String::from("SpriteDefinition"), sprite_definitions_sender.boxed())).await.ok();
        let sprite_transforms   = drawing_properties.send(PropertyRequest::TrackPropertiesWithName(String::from("SpriteTransform"), sprite_transforms_sender.boxed())).await.ok();

        // Receive messages in batches
        let mut messages = messages.ready_chunks(50);

        while let Some(msg_chunk) = messages.next().await {
            for msg in msg_chunk.into_iter() {
                use InternalSpriteLayerRequest::*;

                match msg {
                    SetTransform(new_transform)     => { todo!() }
                    SetLayer(new_layer)             => { todo!() }
                    SetBaseSpriteId(new_sprite)     => { todo!() }
                    SetRefreshRate(refresh_rate)    => { todo!() }
                }
            }
        }
    }).map(|channel| channel.convert_message())
}
