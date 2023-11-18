use flo_render_software::edgeplan::*;
use flo_render_software::edges::*;
use flo_render_software::canvas::*;

use smallvec::*;

#[test]
fn triangle_intercepts() {
    let mut triangle = Polyline::new(vec![
        Coord2(100.0, 100.0),
        Coord2(200.0, 200.0),
        Coord2(300.0, 100.0),
        Coord2(100.0, 100.0),
    ]);
    triangle.prepare_to_render();

    // Get the intercepts that exactly hit the lower line
    let mut intercepts = smallvec![];
    triangle.intercepts_on_line(150.0, &mut intercepts);

    assert!(intercepts.len() == 2, "Should be two intercepts, found {:?}", intercepts);
    assert!(intercepts[0].0 == EdgeInterceptDirection::DirectionOut, "First intercept should be DirectionOut, found {:?}", intercepts);
    assert!(intercepts[1].0 == EdgeInterceptDirection::DirectionIn, "Second intercept should be DirectionIn, found {:?}", intercepts);
}

#[test]
fn horizontal_triangle_line() {
    // If we draw a triangle and then check for intercepts exactly along the bottom line, we should get exactly 2
    // (0 is probably also a valid answer here)
    let mut triangle = Polyline::new(vec![
        Coord2(100.0, 100.0),
        Coord2(200.0, 200.0),
        Coord2(300.0, 100.0),
        Coord2(100.0, 100.0),
    ]);
    triangle.prepare_to_render();

    // Get the intercepts that exactly hit the lower line
    let mut intercepts = smallvec![];
    triangle.intercepts_on_line(100.0, &mut intercepts);

    assert!(intercepts.len() == 2, "Should be two intercepts, found {:?}", intercepts);
    assert!(intercepts[0].0 == EdgeInterceptDirection::DirectionOut, "First intercept should be DirectionOut, found {:?}", intercepts);
    assert!(intercepts[1].0 == EdgeInterceptDirection::DirectionIn, "Second intercept should be DirectionIn, found {:?}", intercepts);
}

#[test]
fn horizontal_triangle_line_reversed() {
    // If we draw a triangle and then check for intercepts exactly along the bottom line, we should get exactly 2
    // (0 is probably also a valid answer here)
    let mut triangle = Polyline::new(vec![
        Coord2(300.0, 100.0),
        Coord2(200.0, 200.0),
        Coord2(100.0, 100.0),
        Coord2(300.0, 100.0),
    ]);
    triangle.prepare_to_render();

    // Get the intercepts that exactly hit the lower line
    let mut intercepts = smallvec![];
    triangle.intercepts_on_line(100.0, &mut intercepts);

    assert!(intercepts.len() == 2, "Should be two intercepts, found {:?}", intercepts);
    assert!(intercepts[0].0 == EdgeInterceptDirection::DirectionOut, "First intercept should be DirectionOut, found {:?}", intercepts);
    assert!(intercepts[1].0 == EdgeInterceptDirection::DirectionIn, "Second intercept should be DirectionIn, found {:?}", intercepts);
}

#[test]
fn double_horizontal_triangle_line() {
    // This time we have a triangle with two horizontal lines that we detect the intercepts for; this should again produce 2 intercepts
    let mut triangle = Polyline::new(vec![
        Coord2(100.0, 100.0),
        Coord2(200.0, 200.0),
        Coord2(300.0, 100.0),
        Coord2(150.0, 100.0),
        Coord2(100.0, 100.0),
    ]);
    triangle.prepare_to_render();

    // Get the intercepts that exactly hit the lower line
    let mut intercepts = smallvec![];
    triangle.intercepts_on_line(100.0, &mut intercepts);

    assert!(intercepts.len() == 2, "Should be two intercepts, found {:?}", intercepts);
    assert!(intercepts[0].0 == EdgeInterceptDirection::DirectionOut, "First intercept should be DirectionOut, found {:?}", intercepts);
    assert!(intercepts[1].0 == EdgeInterceptDirection::DirectionIn, "Second intercept should be DirectionIn, found {:?}", intercepts);
}

#[test]
fn triple_horizontal_triangle_line() {
    // This time we have a triangle with three horizontal lines that we detect the intercepts for; once more, this should produce 2 intercepts
    let mut triangle = Polyline::new(vec![
        Coord2(100.0, 100.0),
        Coord2(200.0, 200.0),
        Coord2(300.0, 100.0),
        Coord2(166.0, 100.0),
        Coord2(133.0, 100.0),
        Coord2(100.0, 100.0),
    ]);
    triangle.prepare_to_render();

    // Get the intercepts that exactly hit the lower line
    let mut intercepts = smallvec![];
    triangle.intercepts_on_line(100.0, &mut intercepts);

    assert!(intercepts.len() == 2, "Should be two intercepts, found {:?}", intercepts);
    assert!(intercepts[0].0 == EdgeInterceptDirection::DirectionOut, "First intercept should be DirectionOut, found {:?}", intercepts);
    assert!(intercepts[1].0 == EdgeInterceptDirection::DirectionIn, "Second intercept should be DirectionIn, found {:?}", intercepts);
}

#[test]
fn triple_horizontal_triangle_line_multiple() {
    // This time we have a triangle with three horizontal lines that we detect the intercepts for; once more, this should produce 2 intercepts
    let mut triangle = Polyline::new(vec![
        Coord2(100.0, 100.0),
        Coord2(200.0, 200.0),
        Coord2(300.0, 100.0),
        Coord2(166.0, 100.0),
        Coord2(133.0, 100.0),
        Coord2(100.0, 100.0),
    ]);
    triangle.prepare_to_render();

    // Get the intercepts that exactly hit the lower line
    let mut intercepts = vec![smallvec![]; 3];
    triangle.intercepts_on_lines(&[99.0, 100.0, 101.0], &mut intercepts);

    assert!(intercepts[0].len() == 0, "Should be zero intercepts, found {:?}", intercepts);

    assert!(intercepts[1].len() == 2, "Should be two intercepts, found {:?}", intercepts);
    assert!(intercepts[1][0].0 == EdgeInterceptDirection::DirectionOut, "First intercept should be DirectionOut, found {:?}", intercepts);
    assert!(intercepts[1][1].0 == EdgeInterceptDirection::DirectionIn, "Second intercept should be DirectionIn, found {:?}", intercepts);

    assert!(intercepts[2].len() == 2, "Should be two intercepts, found {:?}", intercepts);
    assert!(intercepts[2][0].0 == EdgeInterceptDirection::DirectionOut, "First intercept should be DirectionOut, found {:?}", intercepts);
    assert!(intercepts[2][1].0 == EdgeInterceptDirection::DirectionIn, "Second intercept should be DirectionIn, found {:?}", intercepts);
}
