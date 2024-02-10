use crate::wgpu::*;

use futures::prelude::*;
use once_cell::sync::{Lazy};

use flo_scene::*;

use std::sync::*;

/// The scene context used for flo_draw, or None if a scene context has not been created yet
static DRAW_SCENE_CONTEXT: Lazy<Mutex<Option<Arc<Scene>>>> = Lazy::new(|| Mutex::new(None));

///
/// Retrieves or creates a scene for flo_draw
///
pub fn flo_draw_wgpu_scene() -> Arc<Scene> {
    let mut scene = DRAW_SCENE_CONTEXT.lock().unwrap();

    // Start a new scene if none was running
    if scene.is_none() {
        // Create a new scene context, and run it on the winit thread
        let new_scene = Arc::new(Scene::default());

        // Store as the active scene
        *scene = Some(Arc::clone(&new_scene));

        // Run on the winit thread
        winit_thread().send_event(WinitThreadEvent::RunProcess(Box::new(move || async move {
            new_scene.run_scene_with_threads(4).await;
        }.boxed())));
    }

    // Unwrap the scene context
    scene.as_ref().unwrap().clone()
}
