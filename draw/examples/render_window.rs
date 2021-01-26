use flo_draw::*;

use futures::prelude::*;
use futures::executor;

///
/// Simple example that displays a render window and does nothing with it
///
pub fn main() {
    // 'with_2d_graphics' is used to support operating systems that can't run event loops anywhere other than the main thread
    with_2d_graphics(|| {
        // Create a render window and loop until it stops sending events
        executor::block_on(async {
            let (_renderer, mut events) = create_render_window();

            while let Some(_evt) = events.next().await {
            }
        });
    });
}
