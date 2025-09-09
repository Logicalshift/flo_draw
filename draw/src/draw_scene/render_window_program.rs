use crate::draw_scene::*;

#[cfg(all(feature="render-opengl", not(feature="render-wgpu")))]
use super::glutin_render_window_program::*;

#[cfg(feature="render-wgpu")]
use super::wgpu_render_window_program::*;

#[cfg(feature="render-software")]
use super::software_drawing_window_program::*;

use flo_scene::*;

use std::sync::*;

///
/// Creates a render window in a scene for OpenGL rendering with the specified program ID
///
#[cfg(all(feature="render-opengl", not(feature="render-wgpu")))]
pub fn create_render_window_sub_program(scene: &Arc<Scene>, drawing_program: SubProgramId, initial_size: (u64, u64)) -> Result<(), ConnectionError> {
    let render_window_program = SubProgramId::new();

    create_drawing_window_program(scene, drawing_program, render_window_program).unwrap();
    create_glutin_render_window_program(scene, render_window_program, initial_size)
}

///
/// Creates a render window in a scene for WGPU rendering, with a specified program ID
///
#[cfg(all(feature="render-wgpu"))]
pub fn create_render_window_sub_program(scene: &Arc<Scene>, drawing_program: SubProgramId, initial_size: (u64, u64)) -> Result<(), ConnectionError> {
    let render_window_program = SubProgramId::new();

    create_drawing_window_program(scene, drawing_program, render_window_program).unwrap();
    create_wgpu_render_window_program(scene, render_window_program, initial_size)
}

///
/// Creates a drawing window in a scene for software rendering, with a specified program ID
///
/// (Note that a drawing window differs from a render window in that it takes canvas drawing instructions directly instead of render actions)
///
#[cfg(all(feature="render-software", not(any(feature="render-opengl", feature="render-wgpu"))))]
pub fn create_render_window_sub_program(scene: &Arc<Scene>, drawing_program: SubProgramId, initial_size: (u64, u64)) -> Result<(), ConnectionError> {
    let render_window_program = SubProgramId::new();

    create_drawing_window_program(scene, drawing_program, render_window_program).unwrap();
    create_software_draw_window_program(scene, render_window_program, initial_size)
}

///
/// Retrieves or creates a scene context for flo_draw
///
#[cfg(all(not(feature="render-wgpu"), not(feature="render-opengl"), not(feature="render-software")))]
pub fn create_render_window_sub_program(context: &Arc<Scene>, drawing_program: SubProgramId, initial_size: (u64, u64)) -> Result<(), ConnectionError> {
    panic!("No default renderer was specified when flo_draw was compiled (use `render-wgpu` or `render-opengl`)")
}
