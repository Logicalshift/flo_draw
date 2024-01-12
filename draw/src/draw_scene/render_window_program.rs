#[cfg(all(feature="render-opengl", not(feature="render-wgpu")))]
use super::glutin_render_window_program::*;

#[cfg(feature="render-wgpu")]
use super::wgpu_render_window_program::*;

use flo_scene::*;

use std::sync::*;

///
/// Creates a render window in a scene with the specified program ID
///
#[cfg(all(feature="render-opengl", not(feature="render-wgpu")))]
pub fn create_render_window_sub_program(scene: &Arc<Scene>, program_id: SubProgramId, initial_size: (u64, u64)) -> Result<(), ConnectionError> {
    create_glutin_render_window_program(scene, program_id, initial_size)
}

///
/// Retrieves or creates a scene context for flo_draw
///
#[cfg(all(feature="render-wgpu"))]
pub fn create_render_window_sub_program(scene: &Arc<Scene>, program_id: SubProgramId, initial_size: (u64, u64)) -> Result<(), ConnectionError> {
    create_wgpu_render_window_program(scene, program_id, initial_size)
}

///
/// Retrieves or creates a scene context for flo_draw
///
#[cfg(all(not(feature="render-wgpu"), not(feature="render-opengl")))]
pub fn create_render_window_sub_program(context: &Arc<SceneContext>, entity_id: EntityId, initial_size: (u64, u64)) -> Result<(), ConnectionError> {
    panic!("No default renderer was specified when flo_draw was compiled (use `render-wgpu` or `render-opengl`)")
}
