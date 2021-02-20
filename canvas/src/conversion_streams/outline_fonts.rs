use crate::draw::*;

use futures::prelude::*;
use font_kit::*;
use flo_stream::*;

///
/// Given a stream of drawing instructions (such as is returned by `Canvas::stream()`), processes any font or text instructions
/// so that they are removed and replaced with path instructions
///
/// This can be used to render text to a render target that does not have any font support of its own.
///
pub fn stream_outline_fonts<InStream: 'static+Send+Unpin+Stream<Item=Draw>>(draw_stream: InStream) -> impl Send+Unpin+Stream<Item=Draw> {
    generator_stream(move |yield_value| async move {
        // Set up
        let mut draw_stream = draw_stream;

        // Pass through the drawing instructions, and process any font instructions that we may come across
        while let Some(draw) = draw_stream.next().await {
            // TODO: actually process the text instructions :-)
            yield_value(draw).await;
        }
    })
}