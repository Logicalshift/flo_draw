use flo_draw::*;
use flo_canvas::*;

///
/// Demonstrates namespaced layers
///
/// Namespaces when used with layers provide a way to define independent 'layer stacks', which can let different parts of
/// a large program perform their rendering independently without needing to avoid clashing layer IDs (namespaces also apply
/// to other resources like textures, so they also provide a way to manage textures and sprites without risk of clashing
/// IDs)
///
pub fn main() {
    with_2d_graphics(|| {
        // Create a window
        let canvas      = create_drawing_window("Namespaces");

        // We'll create three namespaces that we'll draw overlapping shapes on in an arbitrary order.
        let red_namespace   = NamespaceId::new();
        let green_namespace = NamespaceId::new();
        let blue_namespace  = NamespaceId::new();

        canvas.draw(|gc| {
            // Set up the canvas
            gc.canvas_height(1000.0);
            gc.center_region(0.0, 0.0, 1000.0, 1000.0);

            // Set up the namespace ordering by clearing their Layer 0. Red below green below blue, so things drawn on any layer in the blue namespace appears above the green or red namespace.
            gc.namespace(red_namespace);
            gc.layer(LayerId(0));
            gc.clear_layer();

            gc.namespace(green_namespace);
            gc.layer(LayerId(0));
            gc.clear_layer();

            gc.namespace(blue_namespace);
            gc.layer(LayerId(0));
            gc.clear_layer();

            // Draw a circle on layer 0 in each namesapce
            gc.namespace(red_namespace);
            gc.layer(LayerId(0));
            gc.new_path();
            gc.circle(200.0, 200.0, 100.0);
            gc.fill_color(Color::Rgba(200.0, 0.0, 0.0, 1.0));
            gc.fill();

            gc.namespace(green_namespace);
            gc.layer(LayerId(0));
            gc.new_path();
            gc.circle(210.0, 225.0, 100.0);
            gc.fill_color(Color::Rgba(0.0, 200.0, 0.0, 1.0));
            gc.fill();

            gc.namespace(blue_namespace);
            gc.layer(LayerId(0));
            gc.new_path();
            gc.circle(220.0, 250.0, 100.0);
            gc.fill_color(Color::Rgba(0.0, 0.0, 200.0, 1.0));
            gc.fill();

            // Draw a circle on layer 1 in each namesapce (will appear above the circle but below the shapes in the following namespace)
            gc.namespace(red_namespace);
            gc.layer(LayerId(1));
            gc.new_path();
            gc.rect(270.0, 100.0, 470.0, 300.0);
            gc.fill_color(Color::Rgba(220.0, 0.0, 0.0, 1.0));
            gc.fill();

            gc.namespace(blue_namespace);
            gc.layer(LayerId(1));
            gc.new_path();
            gc.rect(290.0, 150.0, 490.0, 350.0);
            gc.fill_color(Color::Rgba(0.0, 0.0, 220.0, 1.0));
            gc.fill();

            gc.namespace(green_namespace);
            gc.layer(LayerId(1));
            gc.new_path();
            gc.rect(280.0, 125.0, 480.0, 325.0);
            gc.fill_color(Color::Rgba(0.0, 220.0, 0.0, 1.0));
            gc.fill();
        });
    });
}
