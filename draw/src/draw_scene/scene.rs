#[cfg(feature="render-opengl")]
use super::glutin_scene::*;

use flo_scene::*;
use std::sync::*;

///
/// Retrieves or creates a scene context for flo_draw
///
pub fn flo_draw_scene_context() -> Arc<SceneContext> {
    flo_draw_glutin_scene_context()
}
