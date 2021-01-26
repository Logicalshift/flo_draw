use flo_draw::*;

use futures::prelude::*;
use futures::executor;

///
/// Simple example that displays a render window and does nothing with it
///
pub fn main() {
    executor::block_on(async {
        let (_renderer, mut events) = create_render_window();

        while let Some(_evt) = events.next().await {
        }
    })
}
