//!
//! 'Apexes' are the points where a curve changes from moving up to down or vice versa. We use them to determine which lines along an edge
//! require vertical supersampling (for example, the top of the letter 'o' or the crossbar in the letter 'H')
//!
//! These tests use 'real' renderings to check the behaviour of the code that detects apexes.
//!

use flo_render_software::draw::*;
use flo_render_software::edgeplan::*;
use flo_render_software::pixel::*;

use flo_canvas::*;

use futures::prelude::*;
use futures::executor;

use std::sync::*;

static LATO: &[u8] = include_bytes!("../test_data/Lato-Regular.ttf");

///
/// Generates a plan for layer 0 of a drawing at a particular y-position (coordinates are in the -1 to 1 range for a canvas drawing)
///
fn edges_layer_0(instructions: impl 'static + Send + Sync + Iterator<Item=Draw>) -> EdgePlan<Arc<dyn EdgeDescriptor>> {
    // Render the font to paths
    let instructions = stream::iter(instructions);
    let instructions = drawing_with_laid_out_text(instructions);
    let instructions = drawing_with_text_as_paths(instructions);
    let instructions = executor::block_on(async { instructions.collect::<Vec<_>>().await });

    // Draw to the canvas
    let mut drawing = CanvasDrawing::<F32LinearPixel, 4>::empty();
    drawing.set_pixel_height(1080.0);
    drawing.draw(instructions);

    // We'll try to generate the plan for layer 0
    let edges = drawing.edges_for_layer(LayerId(0)).expect("Expected layer 0 to be generated").clone();

    edges
}

#[test]
fn apexes_letter_o() {
    // We'll render a lato 'o' at 16 pixels to generate an edge plan
    let lato = CanvasFontFace::from_slice(LATO);

    let mut drawing = vec![];
    drawing.define_font_data(FontId(1), Arc::clone(&lato));
    drawing.set_font_size(FontId(1), 16.0);
    drawing.draw_text(FontId(1), "o".to_string(), 10.0, 10.0);

    // Edges for the letter 'o'
    let edge_plan = edges_layer_0(drawing.into_iter());

    // Edges for the edge plan should correspond to the letter 'o' (two edges are generated, the inner and outer one)
    let shapes = edge_plan.edges_in_region(0.0..1080.0).collect::<Vec<_>>();
    assert!(shapes.len() == 2, "Expected to find our 'o' shape, but found {:?} shapes instead", shapes.len());

    // Ordering doesn't matter, but there should be two 'o' shapes
    println!("{:?}\n\n", shapes.iter().map(|shape| shape.bounding_box()).collect::<Vec<_>>());

    // Fetch the apexes for this letter
    let mut apexes1 = vec![];
    shapes[0].apexes(&mut apexes1);

    let mut apexes2 = vec![];
    shapes[1].apexes(&mut apexes2);

    // Should be an apex at the top and bottom of each shape
    assert!(apexes1.len() > 0 || apexes2.len() > 0, "Letter 'o' produced no apexes");
    assert!(apexes1.len() > 0, "Produced no apexes for first edge");
    assert!(apexes2.len() > 0, "Produced no apexes for second edge");

    assert!(apexes1.len() >= 2, "First edge should have at least 2 apexes");
    assert!(apexes2.len() >= 2, "Second edge should have at least 2 apexes");

    assert!(apexes1.len() == 2, "First edge should have 2 apexes");
    assert!(apexes2.len() == 2, "Second edge should have 2 apexes");
}
