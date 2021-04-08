use crate::draw::*;

use flo_curves::bezier::*;
use flo_curves::bezier::path::*;
use flo_stream::*;
use futures::prelude::*;

use std::iter;

///
/// Converts a bezier path to a set of paths by a dash patter
///
pub fn path_to_dashed_lines<PathIn, PathOut, DashPattern>(path_in: &PathIn, dash_pattern: DashPattern) -> Vec<PathOut> 
where
PathIn:         BezierPath,
PathOut:        BezierPathFactory<Point=PathIn::Point>,
DashPattern:    Clone+Iterator<Item=f64> {
    // Create the resulting set of paths (most will have just a single curve in them)
    let mut output_paths        = vec![];

    // Cycle the dash pattern
    let mut dash_pattern        = dash_pattern.cycle();

    // Fetch the first length
    let mut remaining_length    = dash_pattern.next().unwrap();

    // We alternate between drawing and not drawing dashes
    let mut draw_dash           = true;

    // Generate dashed lines for each path segment
    let mut start_point         = path_in.start_point();
    let mut current_path_start  = start_point;
    let mut current_path_points = vec![];

    for (cp1, cp2, end_point) in path_in.points() {
        // Create a curve for this section
        let curve                   = Curve::from_points(start_point, (cp1, cp2), end_point);

        if remaining_length <= 0.0 {
            remaining_length        = dash_pattern.next().unwrap();
            draw_dash               = !draw_dash;
        }

        // Walk it, starting with the remaining length and then moving on according to the dash pattern
        let dash_pattern            = &mut dash_pattern;
        let mut dash_pattern_copy   = iter::once(remaining_length).chain(dash_pattern.clone());
        let dash_pattern            = iter::once(remaining_length).chain(dash_pattern);

        for section in walk_curve_evenly(&curve, 1.0, 0.05).vary_by(dash_pattern) {
            // The copied dash pattern will get the expected length for this dash
            let next_length                 = dash_pattern_copy.next().unwrap();

            // walk_curve_evenly uses chord lengths (TODO: arc lengths would be better)
            let section_length              = chord_length(&section);

            // Update the remaining length
            remaining_length                = next_length - section_length;

            // Add the dash to the current path
            let (section_cp1, section_cp2)  = section.control_points();
            let section_end_point           = section.end_point();
            current_path_points.push((section_cp1, section_cp2, section_end_point));

            // If there's enough space for the whole dash, invert the 'draw_dash' state and add the current path to the result
            if remaining_length < 0.1 {
                // Add this dash to the output
                if draw_dash {
                    output_paths.push(PathOut::from_points(current_path_start, current_path_points));
                }

                // Clear the current path
                current_path_start  = section_end_point;
                current_path_points = vec![];

                // Reset for the next dash
                remaining_length    = 0.0;
                draw_dash           = !draw_dash;
            }
        }

        // The start point of the next curve in this path is the end point of this one
        start_point = end_point;
    }

    output_paths
}

///
/// Converts dashed line stroke operations into separate lines
///
pub fn drawing_without_dashed_lines<InStream: 'static+Send+Unpin+Stream<Item=Draw>>(draw_stream: InStream) -> impl Send+Unpin+Stream<Item=Draw> {
    generator_stream(move |yield_value| async move {
        let mut draw_stream = draw_stream;

        // The current path that will be affected
        let mut current_path            = vec![];
        let mut last_point              = Coord2(0.0, 0.0);
        let mut start_point             = Coord2(0.0, 0.0);

        // The dash pattern to apply to the current path
        let mut current_dash_pattern    = None;

        // Stack of stored changes for the paths and dash patterns
        let mut path_stack              = vec![];
        let mut dash_pattern_stack      = vec![];

        while let Some(drawing) = draw_stream.next().await {
            use self::Draw::*;

            match drawing {
                ClearCanvas(colour) => {
                    current_path            = vec![];
                    last_point              = Coord2(0.0, 0.0);
                    start_point             = Coord2(0.0, 0.0);
                    current_dash_pattern    = None;
                    path_stack              = vec![];
                    dash_pattern_stack      = vec![];
                
                    yield_value(ClearCanvas(colour)).await;
                }

                NewPath => {
                    current_path    = vec![];
                    last_point      = Coord2(0.0, 0.0);
                    start_point     = Coord2(0.0, 0.0);

                    yield_value(NewPath).await;
                }

                Move(x, y) => {
                    current_path.push((Coord2(x as _, y as _), vec![]));

                    last_point  = Coord2(x as _, y as _);
                    start_point = Coord2(x as _, y as _);

                    yield_value(Move(x, y)).await;
                }

                Line(x, y) => {
                    let end_point   = Coord2(x as _, y as _);
                    let cp1         = (end_point - last_point) * (1.0/3.0) + last_point;
                    let cp2         = (end_point - last_point) * (2.0/3.0) + last_point;
                    let line        = (cp1, cp2, end_point);

                    current_path.last_mut().map(|path| path.1.push(line));

                    last_point      = Coord2(x as _, y as _);

                    yield_value(Line(x, y)).await;
                }

                BezierCurve((x, y), (cp1x, cp1y), (cp2x, cp2y)) => {
                    let curve = (Coord2(cp1x as _, cp1y as _), Coord2(cp2x as _, cp2y as _), Coord2(x as _, y as _));
                    current_path.last_mut().map(|path| path.1.push(curve));

                    last_point      = Coord2(x as _, y as _);

                    yield_value(BezierCurve((x, y), (cp1x, cp1y), (cp2x, cp2y))).await;
                }

                ClosePath => {
                    let end_point   = start_point;
                    let cp1         = (end_point - last_point) * (1.0/3.0) + last_point;
                    let cp2         = (end_point - last_point) * (2.0/3.0) + last_point;
                    let line        = (cp1, cp2, end_point);

                    current_path.last_mut().map(|path| path.1.push(line));

                    yield_value(ClosePath).await;
                }

                NewDashPattern => {
                    // Invalidate the dash pattern
                    current_dash_pattern = None;
                }

                DashLength(length) => { 
                    // Update the dash pattern
                    current_dash_pattern
                        .get_or_insert_with(|| vec![])
                        .push(length)
                }

                DashOffset(offset) => {
                    // TODO
                }

                PushState => {
                    // Store the current dash pattern and path on the stack
                    path_stack.push(current_path.clone());
                    dash_pattern_stack.push(current_dash_pattern.clone());

                    yield_value(PushState).await;
                }

                PopState => {
                    // Restore the previously stored dash pattern/path
                    current_path            = path_stack.pop().unwrap_or_else(|| vec![]);
                    current_dash_pattern    = dash_pattern_stack.pop().unwrap_or(None);

                    yield_value(PopState).await;
                }

                Stroke => {
                    if let Some(dash_pattern) = &current_dash_pattern {
                        // Create a dash path and pass it through as a new path
                        yield_value(NewPath).await;

                        for subpath in current_path.iter() {
                            for (start_point, curves) in path_to_dashed_lines::<_, SimpleBezierPath, _>(subpath, dash_pattern.iter().map(|p| (*p) as f64)) {
                                yield_value(Move(start_point.x() as _, start_point.y() as _)).await;
                                for (Coord2(cp1x, cp1y), Coord2(cp2x, cp2y), Coord2(x, y)) in curves {
                                    yield_value(BezierCurve((x as _, y as _), (cp1x as _, cp1y as _), (cp2x as _, cp2y as _))).await;
                                }
                            }
                        }

                        // Stroke the dashed line
                        yield_value(Stroke).await;

                        // Restore the original path
                        yield_value(NewPath).await;
                        
                        for (start_point, curves) in current_path.iter() {
                            yield_value(Move(start_point.x() as _, start_point.y() as _)).await;
                            for (Coord2(cp1x, cp1y), Coord2(cp2x, cp2y), Coord2(x, y)) in curves {
                                yield_value(BezierCurve((*x as _, *y as _), (*cp1x as _, *cp1y as _), (*cp2x as _, *cp2y as _))).await;
                            }
                        }
                    } else {
                        // If there's no dash pattern, let the path through untouched
                        yield_value(Stroke).await;
                    }
                }

                drawing => {
                    // Pass the drawing on
                    yield_value(drawing).await;
                }
            }
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