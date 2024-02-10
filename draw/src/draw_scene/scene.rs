#[cfg(feature="render-opengl")]
use super::glutin_scene::*;

#[cfg(feature="render-wgpu")]
use super::wgpu_scene::*;

use flo_scene::*;
use std::sync::*;

///
/// Retrieves or creates a scene context for flo_draw
///
#[cfg(all(feature="render-opengl", not(feature="render-wgpu")))]
pub fn flo_draw_scene_context() -> Arc<Scene> {
    flo_draw_glutin_scene()
}

///
/// Retrieves or creates a scene context for flo_draw
///
#[cfg(all(feature="render-wgpu"))]
pub fn flo_draw_scene_context() -> Arc<Scene> {
    flo_draw_wgpu_scene()
}

///
/// Retrieves or creates a scene context for flo_draw
///
#[cfg(all(not(feature="render-wgpu"), not(feature="render-opengl")))]
pub fn flo_draw_scene_context() -> Arc<SceneContext> {
    panic!("No default renderer was specified when flo_draw was compiled (use `render-wgpu` or `render-opengl`)")
}
