#[cfg(feature="render-opengl")]
use super::glutin_render_window_entity::*;

use flo_scene::*;
use flo_canvas_events::*;

use std::sync::*;

///
/// Creates a render window in a scene with the specified entity ID
///
pub fn create_render_window_entity(context: &Arc<SceneContext>, entity_id: EntityId, initial_size: (u64, u64)) -> Result<SimpleEntityChannel<RenderWindowRequest, ()>, CreateEntityError> {
    create_glutin_render_window_entity(context, entity_id, initial_size)
}

