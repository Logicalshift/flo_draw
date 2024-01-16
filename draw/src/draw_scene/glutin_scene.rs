use crate::glutin::*;

use futures::prelude::*;
use once_cell::sync::Lazy;

use flo_scene::*;

use std::sync::*;

/// The scene context used for flo_draw, or None if a scene context has not been created yet
static DRAW_SCENE_CONTEXT: Lazy<Mutex<Option<Arc<SceneContext>>>> = Lazy::new(|| Mutex::new(None));

///
/// Retrieves or creates a scene context for flo_draw
///
#[allow(dead_code)]
pub fn flo_draw_glutin_scene_context() -> Arc<SceneContext> {
    let mut context = DRAW_SCENE_CONTEXT.lock().unwrap();

    // Start a new scene if none was running
    if context.is_none() {
        // Create a new scene context, and run it on the glutin thread
        let scene       = Scene::default();
        let new_context = scene.context();

        // Run on the glutin thread
        glutin_thread().send_event(GlutinThreadEvent::RunProcess(Box::new(move || async move {
            scene.run().await;
        }.boxed())));

        // Store as the active context
        *context = Some(new_context);
    }

    // Unwrap the scene context
    context.as_ref().unwrap().clone()
}
