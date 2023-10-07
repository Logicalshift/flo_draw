use flo_render_software::edges::*;
use flo_render_software::edgeplan::*;

use smallvec::*;

use std::sync::*;

#[test]
pub fn clip_big_rectangle() {
    // Create a clipping region (square with a hole in it)
    let mut outer_clip_rectangle    = RectangleEdge::new(ShapeId::new(), 100.0..200.0, 200.0..300.0);
    let mut inner_clip_rectangle    = RectangleEdge::new(ShapeId::new(), 125.0..175.0, 225.0..275.0);

    outer_clip_rectangle.prepare_to_render();
    inner_clip_rectangle.prepare_to_render();

    let clip_region = ClipRegion::new(vec![outer_clip_rectangle, inner_clip_rectangle]);
    let clip_region = Arc::new(clip_region);

    // Create a rectangle to clip against this region
    let shape               = RectangleEdge::new(ShapeId::new(), 0.0..400.0, 0.0..400.0);
    let mut clipped_shape   = ClippedShapeEdge::new(ShapeId::new(), clip_region, vec![shape]);

    clipped_shape.prepare_to_render();

    // Check how the shape is clipped
    for y_pos in 0..400 {
        // Get the intercepts at this position
        let y_pos           = y_pos as f64;
        let mut intercepts  = vec![smallvec![]];
        clipped_shape.intercepts(&[y_pos], &mut intercepts);

        if y_pos < 200.0 || y_pos >= 300.0 {
            // Outside the clipping region in y-coordinates
            assert!(intercepts[0].len() == 0, "At ypos {}, intercepts are {:?}", y_pos, intercepts);
        } else if (200.0..225.0).contains(&y_pos) || (275.0..300.0).contains(&y_pos) {
            // Should hit the full range of the rectangle
            assert!(intercepts[0].len() == 2, "At ypos {}, intercepts are {:?}", y_pos, intercepts);
            assert!(intercepts[0][0].1 == 100.0, "At ypos {}, intercepts are {:?}", y_pos, intercepts);
            assert!(intercepts[0][1].1 == 200.0, "At ypos {}, intercepts are {:?}", y_pos, intercepts);
        } else {
            // Other points have a hole in the middle
            assert!(intercepts[0].len() == 4, "At ypos {}, intercepts are {:?}", y_pos, intercepts);
            assert!(intercepts[0][0].1 == 100.0, "At ypos {}, intercepts are {:?}", y_pos, intercepts);
            assert!(intercepts[0][1].1 == 125.0, "At ypos {}, intercepts are {:?}", y_pos, intercepts);
            assert!(intercepts[0][2].1 == 175.0, "At ypos {}, intercepts are {:?}", y_pos, intercepts);
            assert!(intercepts[0][3].1 == 200.0, "At ypos {}, intercepts are {:?}", y_pos, intercepts);
        }
    }
}

#[test]
pub fn clip_inner_rectangle_1() {
    // Create a clipping region (square with a hole in it)
    let mut outer_clip_rectangle    = RectangleEdge::new(ShapeId::new(), 100.0..200.0, 200.0..300.0);
    let mut inner_clip_rectangle    = RectangleEdge::new(ShapeId::new(), 125.0..175.0, 225.0..275.0);

    outer_clip_rectangle.prepare_to_render();
    inner_clip_rectangle.prepare_to_render();

    let clip_region = ClipRegion::new(vec![outer_clip_rectangle, inner_clip_rectangle]);
    let clip_region = Arc::new(clip_region);

    // Create a rectangle to clip against this region
    let shape               = RectangleEdge::new(ShapeId::new(), 110.0..190.0, 0.0..400.0);
    let mut clipped_shape   = ClippedShapeEdge::new(ShapeId::new(), clip_region, vec![shape]);

    clipped_shape.prepare_to_render();

    // Check how the shape is clipped
    for y_pos in 0..400 {
        // Get the intercepts at this position
        let y_pos           = y_pos as f64;
        let mut intercepts  = vec![smallvec![]];
        clipped_shape.intercepts(&[y_pos], &mut intercepts);

        if y_pos < 200.0 || y_pos >= 300.0 {
            // Outside the clipping region in y-coordinates
            assert!(intercepts[0].len() == 0, "At ypos {}, intercepts are {:?}", y_pos, intercepts);
        } else if (200.0..225.0).contains(&y_pos) || (275.0..300.0).contains(&y_pos) {
            // Should hit the full range of the rectangle
            assert!(intercepts[0].len() == 2, "At ypos {}, intercepts are {:?}", y_pos, intercepts);
            assert!(intercepts[0][0].1 == 110.0, "At ypos {}, intercepts are {:?}", y_pos, intercepts);
            assert!(intercepts[0][1].1 == 190.0, "At ypos {}, intercepts are {:?}", y_pos, intercepts);
        } else {
            // Other points have a hole in the middle
            assert!(intercepts[0].len() == 4, "At ypos {}, intercepts are {:?}", y_pos, intercepts);
            assert!(intercepts[0][0].1 == 110.0, "At ypos {}, intercepts are {:?}", y_pos, intercepts);
            assert!(intercepts[0][1].1 == 125.0, "At ypos {}, intercepts are {:?}", y_pos, intercepts);
            assert!(intercepts[0][2].1 == 175.0, "At ypos {}, intercepts are {:?}", y_pos, intercepts);
            assert!(intercepts[0][3].1 == 190.0, "At ypos {}, intercepts are {:?}", y_pos, intercepts);
        }
    }
}

#[test]
pub fn clip_inner_rectangle_2() {
    // Create a clipping region (square with a hole in it)
    let mut outer_clip_rectangle    = RectangleEdge::new(ShapeId::new(), 100.0..200.0, 200.0..300.0);
    let mut inner_clip_rectangle    = RectangleEdge::new(ShapeId::new(), 125.0..175.0, 225.0..275.0);

    outer_clip_rectangle.prepare_to_render();
    inner_clip_rectangle.prepare_to_render();

    let clip_region = ClipRegion::new(vec![outer_clip_rectangle, inner_clip_rectangle]);
    let clip_region = Arc::new(clip_region);

    // Create a rectangle to clip against this region
    let shape               = RectangleEdge::new(ShapeId::new(), 125.0..175.0, 0.0..400.0);
    let mut clipped_shape   = ClippedShapeEdge::new(ShapeId::new(), clip_region, vec![shape]);

    clipped_shape.prepare_to_render();

    // Check how the shape is clipped
    for y_pos in 0..400 {
        // Get the intercepts at this position
        let y_pos           = y_pos as f64;
        let mut intercepts  = vec![smallvec![]];
        clipped_shape.intercepts(&[y_pos], &mut intercepts);

        if y_pos < 200.0 || y_pos >= 300.0 {
            // Outside the clipping region in y-coordinates
            assert!(intercepts[0].len() == 0, "At ypos {}, intercepts are {:?}", y_pos, intercepts);
        } else if (200.0..225.0).contains(&y_pos) || (275.0..300.0).contains(&y_pos) {
            // Should hit the full range of the rectangle
            assert!(intercepts[0].len() == 2, "At ypos {}, intercepts are {:?}", y_pos, intercepts);
            assert!(intercepts[0][0].1 == 125.0, "At ypos {}, intercepts are {:?}", y_pos, intercepts);
            assert!(intercepts[0][1].1 == 175.0, "At ypos {}, intercepts are {:?}", y_pos, intercepts);
        } else {
            // The shape exactly overlaps the hole so there should be nothing inside
            assert!(intercepts[0].len() == 0, "At ypos {}, intercepts are {:?}", y_pos, intercepts);
        }
    }
}

#[test]
pub fn clip_inner_rectangle_3() {
    // Create a clipping region (square with a hole in it)
    let mut outer_clip_rectangle    = RectangleEdge::new(ShapeId::new(), 100.0..200.0, 200.0..300.0);
    let mut inner_clip_rectangle    = RectangleEdge::new(ShapeId::new(), 125.0..175.0, 225.0..275.0);

    outer_clip_rectangle.prepare_to_render();
    inner_clip_rectangle.prepare_to_render();

    let clip_region = ClipRegion::new(vec![outer_clip_rectangle, inner_clip_rectangle]);
    let clip_region = Arc::new(clip_region);

    // Create a rectangle to clip against this region
    let shape               = RectangleEdge::new(ShapeId::new(), 125.0..150.0, 0.0..400.0);
    let mut clipped_shape   = ClippedShapeEdge::new(ShapeId::new(), clip_region, vec![shape]);

    clipped_shape.prepare_to_render();

    // Check how the shape is clipped
    for y_pos in 0..400 {
        // Get the intercepts at this position
        let y_pos           = y_pos as f64;
        let mut intercepts  = vec![smallvec![]];
        clipped_shape.intercepts(&[y_pos], &mut intercepts);

        if y_pos < 200.0 || y_pos >= 300.0 {
            // Outside the clipping region in y-coordinates
            assert!(intercepts[0].len() == 0, "At ypos {}, intercepts are {:?}", y_pos, intercepts);
        } else if (200.0..225.0).contains(&y_pos) || (275.0..300.0).contains(&y_pos) {
            // Should hit the full range of the rectangle
            assert!(intercepts[0].len() == 2, "At ypos {}, intercepts are {:?}", y_pos, intercepts);
            assert!(intercepts[0][0].1 == 125.0, "At ypos {}, intercepts are {:?}", y_pos, intercepts);
            assert!(intercepts[0][1].1 == 150.0, "At ypos {}, intercepts are {:?}", y_pos, intercepts);
        } else {
            // The shape exactly overlaps the hole so there should be nothing inside
            assert!(intercepts[0].len() == 0, "At ypos {}, intercepts are {:?}", y_pos, intercepts);
        }
    }
}

#[test]
pub fn clip_inner_rectangle_4() {
    // Create a clipping region (square with a hole in it)
    let mut outer_clip_rectangle    = RectangleEdge::new(ShapeId::new(), 100.0..200.0, 200.0..300.0);
    let mut inner_clip_rectangle    = RectangleEdge::new(ShapeId::new(), 125.0..175.0, 225.0..275.0);

    outer_clip_rectangle.prepare_to_render();
    inner_clip_rectangle.prepare_to_render();

    let clip_region = ClipRegion::new(vec![outer_clip_rectangle, inner_clip_rectangle]);
    let clip_region = Arc::new(clip_region);

    // Create a rectangle to clip against this region
    let shape               = RectangleEdge::new(ShapeId::new(), 150.0..175.0, 0.0..400.0);
    let mut clipped_shape   = ClippedShapeEdge::new(ShapeId::new(), clip_region, vec![shape]);

    clipped_shape.prepare_to_render();

    // Check how the shape is clipped
    for y_pos in 0..400 {
        // Get the intercepts at this position
        let y_pos           = y_pos as f64;
        let mut intercepts  = vec![smallvec![]];
        clipped_shape.intercepts(&[y_pos], &mut intercepts);

        if y_pos < 200.0 || y_pos >= 300.0 {
            // Outside the clipping region in y-coordinates
            assert!(intercepts[0].len() == 0, "At ypos {}, intercepts are {:?}", y_pos, intercepts);
        } else if (200.0..225.0).contains(&y_pos) || (275.0..300.0).contains(&y_pos) {
            // Should hit the full range of the rectangle
            assert!(intercepts[0].len() == 2, "At ypos {}, intercepts are {:?}", y_pos, intercepts);
            assert!(intercepts[0][0].1 == 150.0, "At ypos {}, intercepts are {:?}", y_pos, intercepts);
            assert!(intercepts[0][1].1 == 175.0, "At ypos {}, intercepts are {:?}", y_pos, intercepts);
        } else {
            // The shape exactly overlaps the hole so there should be nothing inside
            assert!(intercepts[0].len() == 0, "At ypos {}, intercepts are {:?}", y_pos, intercepts);
        }
    }
}
