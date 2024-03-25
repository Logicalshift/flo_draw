use super::canvas_drawing::*;
use super::drawing_state::*;

use crate::pixel::*;
use crate::pixel_programs::*;

use flo_canvas as canvas;

use std::sync::*;

///
/// Definition of a gradient in a canvas drawing
///
#[derive(Clone)]
pub struct Gradient<TPixel> {
    /// Pixels in the gradient, if they've been generated
    gradient_pixels: Option<Arc<Vec<TPixel>>>,

    /// The alpha value set for this gradient
    alpha: f64,

    /// The stops in this gradient
    stops: Vec<(f64, TPixel)>,

    /// True if this gradient is fully opaque
    is_opaque: bool,
}

impl<TPixel, const N: usize> CanvasDrawing<TPixel, N> 
where
    TPixel: 'static + Send + Sync + Pixel<N>,
{
    ///
    /// Performs an operation on a gradient in this drawing
    ///
    pub (super) fn gradient(&mut self, gradient_id: canvas::GradientId, gradient_op: canvas::GradientOp) {
        use canvas::GradientOp::*;

        match gradient_op {
            Create(initial_color)   => self.gradient_create(gradient_id, initial_color),
            AddStop(pos, color)     => self.gradient_add_stop(gradient_id, pos, color),
        }
    }

    ///
    /// Creates/replaces an existing gradient
    ///
    pub (super) fn gradient_create(&mut self, gradient_id: canvas::GradientId, initial_color: canvas::Color) {
        // Convert the colour
        let is_opaque       = initial_color.alpha_component() >= 1.0;
        let initial_color   = TPixel::from_color(initial_color, self.gamma);

        // Create a gradient with 0 stops
        let new_gradient = Gradient {
            gradient_pixels:    None,
            alpha:              1.0,
            stops:              vec![(0.0, initial_color)],
            is_opaque:          is_opaque,
        };

        // Store in this object
        self.gradients.insert((self.current_namespace, gradient_id), new_gradient);
    }

    ///
    /// Adds a colour stop to a gradient that we're building
    ///
    pub (super) fn gradient_add_stop(&mut self, gradient_id: canvas::GradientId, pos: f32, color: canvas::Color) {
        if let Some(gradient) = self.gradients.get_mut(&(self.current_namespace, gradient_id)) {
            let is_opaque = color.alpha_component() >= 1.0;

            // Clear the pixels if any are set so we regenerate the gradient
            gradient.gradient_pixels = None;

            // Add the stop
            gradient.stops.push((pos as f64, TPixel::from_color(color, self.gamma)));
            gradient.is_opaque = gradient.is_opaque && is_opaque;
        }
    }

    ///
    /// True if the specified gradient is opaque
    ///
    #[inline]
    pub (super) fn gradient_is_opaque(&self, namespace_id: canvas::NamespaceId, gradient_id: canvas::GradientId) -> bool {
        if let Some(gradient) = self.gradients.get(&(namespace_id, gradient_id)) {
            gradient.is_opaque
        } else {
            true
        }
    }

    ///
    /// Returns or generates the gradient data for a particular gradient
    ///
    pub (super) fn gradient_data(&mut self, alpha: f64, namespace_id: canvas::NamespaceId, gradient_id: canvas::GradientId, transform: &canvas::Transform2D) -> GradientData<TPixel> {
        let [[a, b, c], [d, e, f], [_, _, _]] = transform.0;
        let transform = [[a as f64, b as _, c as _], [d as _, e as _, f as _]];

        if let Some(gradient) = self.gradients.get_mut(&(namespace_id, gradient_id)) {
            // Generate the stops if needed
            let gradient = if let Some(gradient) = gradient.gradient_pixels.clone() {
                // Gradient already generated
                gradient
            } else {
                // Sort the stops in the gradient
                let stops = &mut gradient.stops;
                stops.sort_by(|(pos1, _), (pos2, _)| pos1.total_cmp(pos2));

                let start_pos   = stops.first().unwrap().0 as f64;
                let end_pos     = stops.last().unwrap().0 as f64;

                if start_pos >= end_pos {
                    // Only one stop or all stops on the same positions
                    Arc::new(vec![stops.first().unwrap().1.clone()])
                } else {
                    // Generate the pixels for this gradient
                    let len             = end_pos - start_pos;

                    let mut pixels      = vec![TPixel::white(); 1024];
                    let mut stops       = stops.iter().cloned();
                    let mut last_stop   = stops.next().unwrap();
                    let mut next_stop   = stops.next().unwrap();

                    for pixel_num in 0..1024 {
                        // Get the x position in 
                        let xpos = pixel_num as f64;
                        let xpos = (xpos / 1024.0) * len + start_pos;

                        // Move on to the next stop while we can (we shouldn't overflow the end as xpos never quite reaches 1024)
                        while next_stop.0 <= xpos {
                            last_stop = next_stop;
                            next_stop = stops.next().unwrap();
                        }

                        // Calculate the pixel at this position by blending between the two stops
                        let diff        = next_stop.0 - last_stop.0;
                        let rel_pos     = xpos - last_stop.0;
                        let fraction    = rel_pos / diff;
                        let fraction    = TPixel::Component::with_value(fraction);

                        pixels[pixel_num] = (next_stop.1 * fraction) + (last_stop.1 * (TPixel::Component::one() - fraction));
                    }

                    // Store the result in the gradient
                    let pixels = Arc::new(pixels);
                    gradient.gradient_pixels = Some(Arc::clone(&pixels));

                    pixels
                }
            };

            // Fill in the data
            GradientData { gradient, alpha, transform }
        } else {
            // Just use the default data
            GradientData {
                gradient:   Arc::new(vec![TPixel::black()]),
                alpha:      alpha,
                transform:  transform,
            }
        }
    }

    ///
    /// Sets the brush to fill using the specified gradient
    ///
    pub (crate) fn fill_gradient(&mut self, gradient_id: canvas::GradientId, x1: f32, y1: f32, x2: f32, y2: f32) {
        let current_state       = &mut self.current_state;
        let program_data_cache  = &mut self.program_data_cache;
        let gradients           = &self.gradients;
        let current_namespace   = self.current_namespace;

        // Transform the coordinates to screen coordinates
        let (x1, y1) = current_state.transform.transform_point(x1, y1);
        let (x2, y2) = current_state.transform.transform_point(x2, y2);

        if let Some(gradient) = gradients.get(&(self.current_namespace, gradient_id)) {
            let fill_alpha = gradient.alpha;

            // Figure out the initial transform
            let transform = canvas::Transform2D::translate(-x1, -y1);
            let transform = canvas::Transform2D::scale(1.0/(x2-x1), 1.0/(y2-y1)) * transform;

            // Release the current fill program
            DrawingState::release_program(&mut current_state.fill_program, program_data_cache);

            // Choose the gradient brush
            current_state.next_fill_brush = Brush::LinearGradient(fill_alpha, current_namespace, gradient_id, transform);
        }
    }
}
