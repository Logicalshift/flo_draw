use crate::draw::*;
use crate::path::*;
use crate::color::*;
use crate::texture::*;
use crate::gradient::*;
use crate::transform2d::*;

use flo_stream::*;
use flo_curves::geo::*;
use flo_curves::bezier::path::*;

use futures::prelude::*;

use std::mem;

///
/// Attributes used to render a bezier path
///
#[derive(Copy, Clone, Debug)]
pub enum PathAttribute {
    /// Path is drawn as a stroke with the specified width and colour
    Stroke(f32, Color),

    /// Path is drawn as a stroke with the specified pixel width and colour
    StrokePixels(f32, Color),

    /// Path is filled with the specified colour
    Fill(Color),

    /// Path is filled with the specified texture
    FillTexture(TextureId, (f32, f32), (f32, f32), Option<Transform2D>),

    /// Path is filled with the specified gradient
    FillGradient(GradientId, (f32, f32), (f32, f32), Option<Transform2D>)
}

///
/// Converts a stream of drawing instructions into a stream of bezier paths (stripping any other attributes from the stream)
///
/// This can generate any path structure that implements the `BezierPathFactory` trait as its result. Bezier paths can't
/// contain 'move' instructions, so the result is a list of path segments that make up the overall path.
///
/// This will return a list of all the paths defined in the specified stream, regardless of if they're actually drawn or
/// used as clipping paths or anything else.
///
pub fn drawing_to_paths<BezierPath, InStream>(draw_stream: InStream) -> impl Send+Unpin+Stream<Item=Vec<BezierPath>>
where
InStream:           'static+Send+Unpin+Stream<Item=Draw>,
BezierPath:         'static+Send+BezierPathFactory,
BezierPath::Point:  Send+Coordinate2D {
    generator_stream(move |yield_value| async move {
        use self::PathOp::*;

        // The path that is currently being created by the path builder (or 'None' if no path has been generated yet)
        let mut current_path: Option<BezierPathBuilder<_>>  = None;
        let mut current_components                          = vec![];
        let mut start_point                                 = None;
        let mut current_transform: Option<Transform2D>      = None;
        let mut transform_stack                             = vec![];

        // Read to the end of the stream
        let mut draw_stream                                 = draw_stream;

        while let Some(draw) = draw_stream.next().await {
            match draw {
                Draw::Path(NewPath)                                             => { 
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

                Draw::Path(Move(x, y))                                          => {
                    // Finish the current path if there is one
                    if let Some(path) = current_path.take() {
                        current_components.push(path.build());
                    }

                    let (x, y) = if let Some(transform) = &current_transform { transform.transform_point(x, y) } else { (x, y) };

                    // Start a new path
                    current_path = Some(BezierPathBuilder::start(BezierPath::Point::from_components(&[x as _, y as _])));
                    start_point  = Some(BezierPath::Point::from_components(&[x as _, y as _]));
                }

                Draw::Path(Line(x, y))                                          => {
                    let (x, y) = if let Some(transform) = &current_transform { transform.transform_point(x, y) } else { (x, y) };

                    // Add the line to the current path
                    current_path = current_path.map(|current_path| {
                        current_path.line_to(BezierPath::Point::from_components(&[x as _, y as _]))
                    });
                }

                Draw::Path(BezierCurve(((cp1x, cp1y), (cp2x, cp2y)), (x1, y1))) => {
                    let ((cp1x, cp1y), (cp2x, cp2y), (x1, y1)) = if let Some(transform) = &current_transform { 
                        (transform.transform_point(cp1x, cp1y), transform.transform_point(cp2x, cp2y), transform.transform_point(x1, y1))
                    } else { 
                        ((cp1x, cp1y), (cp2x, cp2y), (x1, y1))
                    };


                    // Add the curve to the current path
                    current_path = current_path.map(|current_path| {
                        current_path.curve_to(
                            (BezierPath::Point::from_components(&[cp1x as _, cp1y as _]), BezierPath::Point::from_components(&[cp2x as _, cp2y as _])),
                            BezierPath::Point::from_components(&[x1 as _, y1 as _])
                        )
                    });
                }

                Draw::Path(ClosePath)                                           => {
                    if let Some(start_point) = &start_point {
                        current_path = current_path.map(|current_path| {
                            current_path.line_to(start_point.clone())
                        })
                    }
                }

                Draw::MultiplyTransform(transform)                              => {
                    if let Some(last_transform) = current_transform {
                        current_transform = Some(last_transform * transform);
                    } else {
                        current_transform = Some(transform);
                    }
                }

                Draw::PushState                                                 => {
                    transform_stack.push(current_transform);
                }

                Draw::PopState                                                  => {
                    current_transform = transform_stack.pop().unwrap_or(None);
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
///
/// Converts a stream of drawing instructions into a stream of bezier paths with attributes that specify how they're rendered.
///
/// This can generate any path structure that implements the `BezierPathFactory` trait as its result. Bezier paths can't
/// contain 'move' instructions, so the result is a list of path segments that make up the overall path.
///
pub fn drawing_to_attributed_paths<BezierPath, InStream>(draw_stream: InStream) -> impl Send+Unpin+Stream<Item=(Vec<PathAttribute>, Vec<BezierPath>)>
where
InStream:           'static+Send+Unpin+Stream<Item=Draw>,
BezierPath:         'static+Send+BezierPathFactory,
BezierPath::Point:  Send+Coordinate2D {
    generator_stream(move |yield_value| async move {
        use self::PathOp::*;

        // The path that is currently being created by the path builder (or 'None' if no path has been generated yet)
        let mut current_path: Option<BezierPathBuilder<_>>  = None;
        let mut current_components                          = vec![];
        let mut current_attributes                          = vec![];
        let mut start_point                                 = None;

        let mut fill_color                                  = PathAttribute::Fill(Color::Rgba(0.0, 0.0, 0.0, 1.0));
        let mut stroke_color                                = Color::Rgba(0.0, 0.0, 0.0, 1.0);
        let mut line_width                                  = None;
        let mut line_width_pixels                           = None;
        let mut current_transform: Option<Transform2D>      = None;

        let mut state_stack                                 = vec![];

        // Read to the end of the stream
        let mut draw_stream                                 = draw_stream;

        while let Some(draw) = draw_stream.next().await {
            match draw {
                Draw::Path(NewPath)                                             => { 
                    // Add the last path to the list of path components
                    if let Some(path) = current_path.take() {
                        current_components.push(path.build());
                    }

                    // If there are any components to the current path, return them
                    if current_components.len() > 0 && current_attributes.len() > 0 {
                        let next_path       = mem::take(&mut current_components);
                        let next_attributes = mem::take(&mut current_attributes);

                        yield_value((next_attributes, next_path)).await;
                    }

                    current_attributes = vec![];
                    current_components = vec![];
                }

                Draw::Path(Move(x, y))                                          => {
                    // Finish the current path if there is one
                    if let Some(path) = current_path.take() {
                        current_components.push(path.build());
                    }

                    let (x, y) = if let Some(transform) = &current_transform { transform.transform_point(x, y) } else { (x, y) };

                    // Start a new path
                    current_path = Some(BezierPathBuilder::start(BezierPath::Point::from_components(&[x as _, y as _])));
                    start_point  = Some(BezierPath::Point::from_components(&[x as _, y as _]));
                }

                Draw::Path(Line(x, y))                                          => {
                    let (x, y) = if let Some(transform) = &current_transform { transform.transform_point(x, y) } else { (x, y) };

                    // Add the line to the current path
                    current_path = current_path.map(|current_path| {
                        current_path.line_to(BezierPath::Point::from_components(&[x as _, y as _]))
                    });
                }

                Draw::Path(BezierCurve(((cp1x, cp1y), (cp2x, cp2y)), (x1, y1))) => {
                    let ((cp1x, cp1y), (cp2x, cp2y), (x1, y1)) = if let Some(transform) = &current_transform { 
                        (transform.transform_point(cp1x, cp1y), transform.transform_point(cp2x, cp2y), transform.transform_point(x1, y1))
                    } else { 
                        ((cp1x, cp1y), (cp2x, cp2y), (x1, y1))
                    };

                    // Add the curve to the current path
                    current_path = current_path.map(|current_path| {
                        current_path.curve_to(
                            (BezierPath::Point::from_components(&[cp1x as _, cp1y as _]), BezierPath::Point::from_components(&[cp2x as _, cp2y as _])),
                            BezierPath::Point::from_components(&[x1 as _, y1 as _])
                        )
                    });
                }

                Draw::Path(ClosePath)                                           => {
                    if let Some(start_point) = &start_point {
                        current_path = current_path.map(|current_path| {
                            current_path.line_to(start_point.clone())
                        })
                    }
                }

                Draw::FillColor(new_fill_color)                                 => {
                    fill_color = PathAttribute::Fill(new_fill_color);
                }

                Draw::FillGradient(gradient, (x1, y1), (x2, y2))                => {
                    fill_color = PathAttribute::FillGradient(gradient, (x1, y1), (x2, y2), None);
                }

                Draw::FillTexture(texture, (x1, y1), (x2, y2))                  => {
                    fill_color = PathAttribute::FillTexture(texture, (x1, y1), (x2, y2), None);
                }

                Draw::FillTransform(transform)                                  => {
                    fill_color = match fill_color {
                        PathAttribute::FillGradient(gradient, coord1, coord2, None)                     => PathAttribute::FillGradient(gradient, coord1, coord2, Some(transform)),
                        PathAttribute::FillTexture(texture, coord1, coord2, None)                       => PathAttribute::FillTexture(texture, coord1, coord2, Some(transform)),
                        PathAttribute::FillGradient(gradient, coord1, coord2, Some(existing_transform)) => PathAttribute::FillGradient(gradient, coord1, coord2, Some(existing_transform * transform)),
                        PathAttribute::FillTexture(texture, coord1, coord2, Some(existing_transform))   => PathAttribute::FillTexture(texture, coord1, coord2, Some(existing_transform * transform)),

                        other_fill_color                                                                => other_fill_color
                    };
                }

                Draw::StrokeColor(new_stroke_color)                             => {
                    stroke_color = new_stroke_color;
                }

                Draw::LineWidth(new_line_width)                                 => {
                    line_width_pixels   = None;
                    line_width          = Some(new_line_width);
                }

                Draw::LineWidthPixels(new_line_width)                           => {
                    line_width_pixels   = Some(new_line_width);
                    line_width          = None;
                }

                Draw::MultiplyTransform(transform)                              => {
                    if let Some(last_transform) = current_transform {
                        current_transform = Some(last_transform * transform);
                    } else {
                        current_transform = Some(transform);
                    }
                }

                Draw::PushState                                                 => {
                    state_stack.push((fill_color, stroke_color, line_width, line_width_pixels, current_transform));
                }

                Draw::PopState                                                  => {
                    if let Some((new_fill_color, new_stroke_color, new_line_width, new_line_width_pixels, new_transform)) = state_stack.pop() {
                        fill_color          = new_fill_color;
                        stroke_color        = new_stroke_color;
                        line_width          = new_line_width;
                        line_width_pixels   = new_line_width_pixels;
                        current_transform   = new_transform;
                    }
                }

                Draw::Fill                                                      => {
                    current_attributes.push(fill_color);
                }

                Draw::Stroke                                                    => {
                    if let Some(line_width) = line_width {
                        current_attributes.push(PathAttribute::Stroke(line_width, stroke_color));
                    }
                    if let Some(line_width_pixels) = line_width_pixels {
                        current_attributes.push(PathAttribute::StrokePixels(line_width_pixels, stroke_color));
                    }
                }

                // Ignore other instructions
                _                                                               => { }
            }
        }

        // Return the last path once the stream has finished
        if let Some(path) = current_path.take() {
            current_components.push(path.build());
        }

        // If there are any components to the current path, return them
        if current_components.len() > 0 && current_attributes.len() > 0 {
            let next_path       = mem::take(&mut current_components);
            let next_attributes = mem::take(&mut current_attributes);

            yield_value((next_attributes, next_path)).await;
        }
    })
}

#[cfg(test)]
mod test {
    use super::*;
    use futures::stream;
    use futures::executor;

    #[test]
    pub fn square_path() {
        executor::block_on(async {
            // Describe a square
            let square          = vec![
                Draw::Path(PathOp::NewPath),
                Draw::Path(PathOp::Move(0.0, 0.0)), 
                Draw::Path(PathOp::Line(100.0, 0.0)), 
                Draw::Path(PathOp::Line(100.0, 100.0)), 
                Draw::Path(PathOp::Line(0.0, 100.0)), 
                Draw::Path(PathOp::ClosePath)
            ];

            // Stream it through drawing_to_paths
            let square_stream   = stream::iter(square);
            let path_stream     = drawing_to_paths::<SimpleBezierPath, _>(square_stream);
            
            // Collect the paths that result
            let paths           = path_stream.collect::<Vec<_>>().await;

            // Should contain our square
            assert!(paths.len() == 1);
            assert!(paths[0].len() == 1);

            let (start, curves) = &paths[0][0];

            assert!(start == &Coord2(0.0, 0.0));
            assert!(curves[0].2 == Coord2(100.0, 0.0));
            assert!(curves[1].2 == Coord2(100.0, 100.0));
            assert!(curves[2].2 == Coord2(0.0, 100.0));
            assert!(curves[3].2 == Coord2(0.0, 0.0));

            assert!(curves.len() == 4);
        });
    }
}