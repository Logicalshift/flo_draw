use crate::glutin::*;

use futures::prelude::*;
use once_cell::sync::Lazy;

use flo_scene::*;

use std::sync::*;

/// The scene context used for flo_draw, or None if a scene context has not been created yet
static DRAW_SCENE: Lazy<Mutex<Option<Arc<Scene>>>> = Lazy::new(|| Mutex::new(None));

///
/// Retrieves or creates a scene for flo_draw
///
#[allow(dead_code)]
pub fn flo_draw_glutin_scene() -> Arc<Scene> {
    let mut scene = DRAW_SCENE.lock().unwrap();

    // Start a new scene if none was running
    if scene.is_none() {
        // Create a new scene, and run it on the glutin thread
        let new_scene = Arc::new(Scene::default());

        // Store as the active scene
        *scene = Some(Arc::clone(&new_scene));

        // Run on the glutin thread
        glutin_thread().send_event(GlutinThreadEvent::RunProcess(Box::new(move || async move {
            new_scene.run_scene_with_threads(4).await;
        }.boxed())));
    }

    // Unwrap the scene context
    scene.as_ref().unwrap().clone()
}
