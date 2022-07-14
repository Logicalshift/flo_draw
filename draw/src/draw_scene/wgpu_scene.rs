use crate::wgpu::*;

use futures::prelude::*;

use flo_scene::*;

use std::sync::*;

lazy_static! {
    /// The scene context used for flo_draw, or None if a scene context has not been created yet
    static ref DRAW_SCENE_CONTEXT: Mutex<Option<Arc<SceneContext>>> = Mutex::new(None);
}

///
/// Retrieves or creates a scene context for flo_draw
///
pub fn flo_draw_wgpu_scene_context() -> Arc<SceneContext> {
    let mut context = DRAW_SCENE_CONTEXT.lock().unwrap();

    // Start a new scene if none was running
    if context.is_none() {
        // Create a new scene context, and run it on the winit thread
        let scene       = Scene::default();
        let new_context = scene.context();

        // Run on the winit thread
        winit_thread().send_event(WinitThreadEvent::RunProcess(Box::new(move || async move {
            scene.run().await;
        }.boxed())));

        // Store as the active context
        *context = Some(new_context);
    }

    // Unwrap the scene context
    context.as_ref().unwrap().clone()
}
