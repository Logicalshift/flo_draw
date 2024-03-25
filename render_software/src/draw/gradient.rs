use super::canvas_drawing::*;

use crate::pixel::*;

use flo_canvas as canvas;

use std::sync::*;

///
/// Definition of a gradient in a canvas drawing
///
#[derive(Clone)]
pub struct Gradient<TPixel> {
    /// Pixels in the gradient, if they've been generated
    gradient_pixels: Option<Arc<Vec<TPixel>>>,

    /// The stops in this gradient
    stops: Vec<(f32, TPixel)>,
}

impl<TPixel, const N: usize> CanvasDrawing<TPixel, N> 
where
    TPixel: 'static + Send + Sync + Pixel<N>,
{
    ///
    /// Performs an operation on a gradient in this drawing
    ///
    pub fn gradient(&mut self, gradient_id: canvas::GradientId, gradient_op: canvas::GradientOp) {
        use canvas::GradientOp::*;

        match gradient_op {
            Create(initial_color)   => self.gradient_create(gradient_id, initial_color),
            AddStop(pos, color)     => self.gradient_add_stop(gradient_id, pos, color),
        }
    }

    ///
    /// Creates/replaces an existing gradient
    ///
    pub fn gradient_create(&mut self, gradient_id: canvas::GradientId, initial_color: canvas::Color) {
        // Convert the colour
        let initial_color = TPixel::from_color(initial_color, self.gamma);

        // Create a gradient with 0 stops
        let new_gradient = Gradient {
            gradient_pixels:    None,
            stops:              vec![(0.0, initial_color)],
        };

        // Store in this object
        self.gradients.insert((self.current_namespace, gradient_id), new_gradient);
    }

    ///
    /// Adds a colour stop to a gradient that we're building
    ///
    pub fn gradient_add_stop(&mut self, gradient_id: canvas::GradientId, pos: f32, color: canvas::Color) {
        if let Some(gradient) = self.gradients.get_mut(&(self.current_namespace, gradient_id)) {
            // Clear the pixels if any are set so we regenerate the gradient
            gradient.gradient_pixels = None;

            // Add the stop
            gradient.stops.push((pos, TPixel::from_color(color, self.gamma)));
        }
    }
}
