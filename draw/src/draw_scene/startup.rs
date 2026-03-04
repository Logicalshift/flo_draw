use crate::platform::*;

use flo_scene::*;
use flo_stream::*;

///
/// Sets up the standard programs that run in a flo_draw scene
///
pub fn start_flo_draw_scene(scene: &Scene) {
    start_platform(scene);
}

#[cfg(target_os = "macos")]
fn start_platform(scene: &Scene) {
    let winit_events = winit_events().subscribe();

    scene.add_subprogram(SubProgramId::called("flo_draw::macos::platform"), move |input, context| macos_platform_subprogram(input, context, winit_events), 1);
}

#[cfg(all(not(target_os = "macos")))]
fn start_platform(_scene: &Scene) {
    // Platform with no custom startup options
}
