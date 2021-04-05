use crate::draw::*;

use flo_curves::*;
use flo_stream::*;
use futures::prelude::*;

///
/// Converts dashed line stroke operations into separate lines
///
pub fn drawing_without_dashed_lines<InStream: 'static+Send+Unpin+Stream<Item=Draw>>(draw_stream: InStream) -> impl Send+Unpin+Stream<Item=Draw> {
    generator_stream(move |yield_value| async move {
        let mut draw_stream = draw_stream;

        while let Some(drawing) = draw_stream.next().await {
            // Pass the drawing on
            yield_value(drawing).await;
        }
    })
}

#[cfg(test)]
mod test {
    use super::*;

    use futures::stream;
    use futures::executor;

    #[test]
    fn pass_through_normal_path() {
        let input_drawing = vec![
            Draw::NewPath,
            Draw::Move(10.0, 10.0),
            Draw::Line(10.0, 100.0),
            Draw::Line(100.0, 100.0),
            Draw::Line(100.0, 10.0),
            Draw::ClosePath
        ];

        executor::block_on(async move {
            let without_dashed_lines    = drawing_without_dashed_lines(stream::iter(input_drawing.into_iter()));
            let output_drawing          = without_dashed_lines.collect::<Vec<_>>().await;

            assert!(output_drawing == vec![
                Draw::NewPath,
                Draw::Move(10.0, 10.0),
                Draw::Line(10.0, 100.0),
                Draw::Line(100.0, 100.0),
                Draw::Line(100.0, 10.0),
                Draw::ClosePath
            ]);
        });
    }
}