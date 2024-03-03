use flo_render_software::edgeplan::*;
use flo_render_software::edges::*;
use flo_render_software::scanplan::*;
use flo_render_software::canvas::*;

#[test]
fn scan_triangle() {
    let mut triangle = Polyline::new(vec![
        Coord2(100.0, 100.0),
        Coord2(200.0, 200.0),
        Coord2(300.0, 100.0),
        Coord2(100.0, 100.0),
    ]).to_non_zero_edge(ShapeId::new());
    triangle.prepare_to_render();

    // Iterate across the triangle to get a series of shards
    let shards = shard_intercepts_from_edge(&triangle, 
        &[99.0, 100.0, 125.0, 150.0, 175.0, 200.0],
        &[100.0, 125.0, 150.0, 175.0, 200.0, 201.0])
        .collect::<Vec<_>>();

    println!("{:?}", shards);
    assert!(shards.len() == 6, "Should be 6 shards {:?}", shards);

    // 99.0-100.0 should be empty
    assert!(shards[0].len() == 0, "99-100 should have no intercepts ({:?})", shards);

    // 100.0-125.0 should also be empty. Our triangle has a x=y gradient
    assert!(shards[1].len() == 2, "100-125 should have 2 intercepts ({:?})", shards);
    assert!((shards[1][0].x_range().start-100.0).abs() < 0.01, "100-125 should start at 100 ({:?})", shards);
    assert!((shards[1][0].x_range().end-125.0).abs() < 0.01, "100-125 should end at 125 ({:?})", shards);
    assert!((shards[1][1].x_range().start-275.0).abs() < 0.01, "100-125 should start at 275 ({:?})", shards);
    assert!((shards[1][1].x_range().end-300.0).abs() < 0.01, "100-125 should end at 300 ({:?})", shards);

    // There are no intercepts at 200, so all the shards except the last two should have 2 intercepts
    assert!(shards[2].len() == 2, "125-150 should have 2 intercepts ({:?})", shards);
    assert!(shards[3].len() == 2, "150-175 should have 2 intercepts ({:?})", shards);
    assert!(shards[4].len() == 0, "175-200 should have 0 intercepts ({:?})", shards);
    assert!(shards[5].len() == 0, "200-201 should have 0 intercepts ({:?})", shards);
}

#[test]
fn scan_concave() {
    // This is a simple concave shape that needs some additional processing to render correctly
    let mut concave_shape = Polyline::new(vec![
        Coord2(100.0, 100.0),
        Coord2(150.0, 200.0),
        Coord2(200.0, 150.0),
        Coord2(250.0, 200.0),
        Coord2(300.0, 100.0),
        Coord2(100.0, 100.0),
    ]).to_non_zero_edge(ShapeId::new());
    concave_shape.prepare_to_render();

    // Iterate across the shape to get a series of shards
    let shards = shard_intercepts_from_edge(&concave_shape, 
        &[99.0, 100.0, 125.0, 150.0, 175.0, 198.0, 199.0, 200.0],
        &[100.0, 125.0, 150.0, 175.0, 198.0, 199.0, 200.0, 201.0])
        .collect::<Vec<_>>();

    println!("{:?}", shards);
    assert!(shards.len() == 8, "Should be 8 shards {:?}", shards);

    // 99.0-100.0 should be empty
    assert!(shards[0].len() == 0, "99-100 should have no intercepts ({:?})", shards);
}

#[test]
fn scan_disjointed() {
    // This shape can confuse the algorithm, as we'll generate two slices that have different gaps in them
    //
    //    +-      /---+
    //    | \    /    |
    //    |  \  /     |
    // -> |   \/      |
    // -> |        /\ |
    //    |       /  \|
    //    +-------    +

    let mut concave_shape = Polyline::new(vec![
        Coord2(0.0, 0.0),
        Coord2(0.0, 100.0),
        Coord2(10.0, 100.0),
        Coord2(50.0, 50.0),
        Coord2(100.0, 100.0),
        Coord2(150.0, 100.0),
        Coord2(150.0, 100.0),
        Coord2(150.0, 0.0),
        Coord2(125.0, 50.0),
        Coord2(75.0, 0.0),
        Coord2(0.0, 0.0),
    ]).to_non_zero_edge(ShapeId::new());
    concave_shape.prepare_to_render();

    // Get the shards for the conflicting region. 49 is one set of intersections, 51 is another
    let shards = shard_intercepts_from_edge(&concave_shape, 
        &[49.0],
        &[51.0])
        .collect::<Vec<_>>();

    println!("{:?}", shards);

    // 99.0-100.0 should be empty
    assert!(shards[0].len() == 2, "49-51 should have only two intercepts ({:?})", shards);

}
