use crate::draw::*;

use flo_stream::*;
use flo_curves::geo::*;
use flo_curves::bezier::path::*;

use futures::prelude::*;

use std::mem;

///
/// Converts a stream of drawing instructions into a stream of bezier paths (stripping any other attributes from the stream)
///
/// This can generate any path structure that implements the `BezierPathFactory` trait as its result. Bezier paths can't
/// contain 'move' instructions, so the result is a list of path segments that make up the overall path.
///
pub fn to_paths<BezierPath, InStream>(draw_stream: InStream) -> impl Send+Unpin+Stream<Item=Vec<BezierPath>>
where
InStream:           'static+Send+Unpin+Stream<Item=Draw>,
BezierPath:         'static+Send+BezierPathFactory,
BezierPath::Point:  Send+Coordinate2D {
    generator_stream(move |yield_value| async move {
        // The path that is currently being created by the path builder (or 'None' if no path has been generated yet)
        let mut current_path: Option<BezierPathBuilder<_>>  = None;
        let mut current_components                          = vec![];

        // Read to the end of the stream
        let mut draw_stream                                 = draw_stream;

        while let Some(draw) = draw_stream.next().await {
            match draw {
                Draw::NewPath                                   => { 
                    // Add the last path to the list of path components
                    if let Some(path) = current_path.take() {
                        current_components.push(path.build());
                    }

                    // If there are any components to the current path, return them
                    if current_components.len() > 0 {
                        let mut next_path = vec![];
                        mem::swap(&mut next_path, &mut current_components);

                        yield_value(next_path).await;
                    }
                }

                Draw::Move(x, y)                                => {
                    // Finish the current path if there is one
                    if let Some(path) = current_path.take() {
                        current_components.push(path.build());
                    }

                    // Start a new path
                    current_path = Some(BezierPathBuilder::start(BezierPath::Point::from_components(&[x as _, y as _])));
                }

                Draw::Line(x, y)                                => {
                    // Add the line to the current path
                    current_path = current_path.map(|current_path| {
                        current_path.line_to(BezierPath::Point::from_components(&[x as _, y as _]))
                    });
                }

                Draw::BezierCurve((x1, y1), (x2, y2), (x3, y3)) => {
                    // Add the curve to the current path
                    current_path = current_path.map(|current_path| {
                        current_path.curve_to(
                            (BezierPath::Point::from_components(&[x2 as _, y2 as _]), BezierPath::Point::from_components(&[x3 as _, y3 as _])),
                            BezierPath::Point::from_components(&[x1 as _, y1 as _])
                        )
                    });
                }

                // Ignore other instructions
                _ => { }
            }
        }

        // Return the last path once the stream has finished
        if let Some(path) = current_path.take() {
            current_components.push(path.build());
        }

        // If there are any components to the current path, return them
        if current_components.len() > 0 {
            let mut next_path = vec![];
            mem::swap(&mut next_path, &mut current_components);

            yield_value(next_path).await;
        }
    })
}
