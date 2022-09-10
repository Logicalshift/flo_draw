use super::drawing_request::*;
use crate::*;

use flo_scene::*;

use std::time::{Duration};
use std::sync::*;

// TODO: how to deal with things like updating the canvas transform/transforming all the sprites?
// TODO: might be better to work by taking drawing requests and sending them on? Or could have some commands that update the sprite layer's transform independently.

///
/// Request for the sprite properties layer
///
#[derive(Clone, Debug)]
pub enum SpriteLayerRequest {
    /// Sets the transform applied to the sprites layer
    SetTransform(Transform2D),

    /// Changes the sprite layer (and re-renders)
    SetLayer(LayerId),

    /// Changes the base sprite ID (forcing a re-render of the sprites)
    SetBaseSpriteId(SpriteId),

    /// Changes the refresh rate (time between an invalidatation and a canvas update) of the sprite layer
    SetRefreshRate(Duration),
}

///
/// Creates a sprite layer entity in a scene
///
/// This will monitor the properties for all other entities in a scene for the `SpriteDefinition` property (of type `Vec<Draw>`) and `
/// SpriteTransform` property (of type `SpriteTransform`, `Option<SpriteTransform>` or `Vec<SpriteTransform>`). The SpriteDefinition is
/// used to define and update a sprite in the canvas, and the sprite is then drawn on the sprite layer at the position(s) specified by
/// the sprite transform.
///
/// Optionally the property `SpriteZIndex`, of type `f64` can be used for sorting purposes.
///
/// The sprite layer uses its own transform for rendering, which can be adjusted by sending commands to the sprite layer entity.
///
pub fn create_sprite_layer_entity(entity_id: EntityId, context: &Arc<SceneContext>, initial_sprite_id: SpriteId, sprite_layer: LayerId, canvas: impl EntityChannel<Message=DrawingRequest>) -> Result<impl EntityChannel<Message=SpriteLayerRequest, CreateEntityError> {
    unimplemented!()
}
