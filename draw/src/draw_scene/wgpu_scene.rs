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

        // When tokio support is enabled, we need to run the scene within a tokio runtime
        #[cfg(feature = "tokio_support")]
        {
            let tokio_runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();

            winit_thread().send_event(WinitThreadEvent::RunProcess(Box::new(move || async move {
                use std::pin::{pin};
                let mut run_scene = pin!(new_scene.run_scene_with_threads(4));

                future::poll_fn(move |ctxt| {
                    let in_runtime  = tokio_runtime.enter();
                    let result      = run_scene.poll_unpin(ctxt);
                    drop(in_runtime);

                    result
                }).await;
            }.boxed()), "DrawWGPUScene".into()));
        }

        // Without tokio support, run on the winit thread directly
        #[cfg(not(feature = "tokio_support"))]
        winit_thread().send_event(WinitThreadEvent::RunProcess(Box::new(move || async move {
            new_scene.run_scene_with_threads(4).await;
        }.boxed()), "DrawWGPUScene".into()));
    }

    // Unwrap the scene context
    scene.as_ref().unwrap().clone()
}
