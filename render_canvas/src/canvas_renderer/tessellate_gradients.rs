use super::canvas_renderer::*;

use crate::render_gradient::*;

use flo_canvas as canvas;

impl CanvasRenderer {
    ///
    /// Carries out a gradient operation
    ///
    #[inline]
    pub (super) fn tes_gradient(&mut self, gradient_id: canvas::GradientId, op: canvas::GradientOp) {
        use canvas::GradientOp::*;

        match op {
            Create(initial_colour)      => self.tes_gradient_create(gradient_id, initial_colour),
            AddStop(pos, stop_colour)   => self.tes_gradient_add_stop(gradient_id, pos, stop_colour),
        }
    }

    ///
    /// Start a new gradient definition
    ///
    pub (super) fn tes_gradient_create(&mut self, gradient_id: canvas::GradientId, initial_colour: canvas::Color) {
        self.core.sync(move |core| {
            core.canvas_gradients.insert(gradient_id, RenderGradient::Defined(vec![canvas::GradientOp::Create(initial_colour)]));
        });
    }

    ///
    /// Add a stop to an existing gradient definition
    ///
    pub (super) fn tes_gradient_add_stop(&mut self, gradient_id: canvas::GradientId, pos: f32, stop_colour: canvas::Color) {
        self.core.sync(move |core| {
            use canvas::GradientOp::AddStop;

            match core.canvas_gradients.get_mut(&gradient_id) {
                Some(RenderGradient::Defined(defn)) => {
                    // Gradient has not yet been mapped to a texture
                    defn.push(AddStop(pos, stop_colour))
                }

                Some(RenderGradient::Ready(_, defn)) => {
                    // Gradient has been mapped to a texture (continue defining it as a new texture)
                    let mut defn = defn.clone();
                    defn.push(AddStop(pos, stop_colour));
                    core.canvas_gradients.insert(gradient_id, RenderGradient::Defined(defn));
                }

                None => { }
            }
        });
    }
}
