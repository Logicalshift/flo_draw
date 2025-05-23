use super::offscreen_trait::*;

use flo_canvas::*;

use futures::prelude::*;

///
/// Renders a canvas in an offscreen context, returning the resulting bitmap
///
pub fn render_canvas_offscreen<'a>(context: &'a mut impl OffscreenDrawingContext, width: usize, height: usize, actions: impl 'a + Stream<Item=Draw>) -> impl 'a+Future<Output=Vec<u8>> {
    async move {
        // Perform as many drawing actions simultaneously as we can
        let actions             = Box::pin(actions);
        let mut actions         = actions.ready_chunks(10000);

        // Create the offscreen render target
        let mut drawing_target  = context.create_drawing_target(width, height);

        // Send the drawing instructions from the action stream
        while let Some(drawing) = actions.next().await {
            // Render the next set of actions
            drawing_target.draw_actions(drawing).await;
        }

        // Result is the realized rendering
        drawing_target.realize()
    }
}