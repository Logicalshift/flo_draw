use crate::fill_state::*;
use crate::render_entity::*;

use super::canvas_renderer::*;

use flo_canvas as canvas;
use flo_render as render;

use lyon::tessellation::{FillRule};

impl CanvasRenderer {
    ///
    /// Converts a canvas colour to a render colour
    ///
    pub (super) fn render_color(color: canvas::Color) -> render::Rgba8 {
        let (r, g, b, a)    = color.to_rgba_components();
        let (r, g, b, a)    = (Self::col_to_u8(r), Self::col_to_u8(g), Self::col_to_u8(b), Self::col_to_u8(a));

        render::Rgba8([r, g, b, a])
    }

    ///
    /// Changes a colour component to a u8 format
    ///
    pub (super) fn col_to_u8(component: f32) -> u8 {
        if component > 1.0 {
            255
        } else if component < 0.0 {
            0
        } else {
            (component * 255.0) as u8
        }
    }

    /// Set the line width
    #[inline]
    pub (super) fn tes_line_width(&mut self, width: f32) {
        self.core.sync(|core| core.layer(self.current_layer).state.stroke_settings.line_width = width);
    }

    /// Set the line width in pixels
    #[inline]
    pub (super) fn tes_line_width_pixels(&mut self, pixel_width: f32) {
        // TODO: if the window width changes we won't re-tessellate the lines affected by this line width
        let canvas::Transform2D(transform)  = &self.active_transform;
        let pixel_size                      = 2.0/self.window_size.1 * self.window_scale;
        let pixel_width                     = pixel_width * pixel_size;
        let scale                           = (transform[0][0]*transform[0][0] + transform[1][0]*transform[1][0]).sqrt();
        let width                           = pixel_width / scale;

        self.core.sync(|core| core.layer(self.current_layer).state.stroke_settings.line_width = width);
    }

    /// Line join
    #[inline]
    pub (super) fn tes_line_join(&mut self, join_type: canvas::LineJoin) {
        self.core.sync(|core| core.layer(self.current_layer).state.stroke_settings.join = join_type);
    }

    /// The cap to use on lines
    #[inline]
    pub (super) fn tes_line_cap(&mut self, cap_type: canvas::LineCap) {
        self.core.sync(|core| core.layer(self.current_layer).state.stroke_settings.cap = cap_type);
    }

    /// The winding rule to use when filling areas
    #[inline]
    pub (super) fn tes_winding_rule(&mut self, winding_rule: canvas::WindingRule) {
        use canvas::WindingRule::*;

        match winding_rule {
            EvenOdd     => self.core.sync(|core| core.layer(self.current_layer).state.winding_rule = FillRule::EvenOdd),
            NonZero     => self.core.sync(|core| core.layer(self.current_layer).state.winding_rule = FillRule::NonZero)
        }
        
    }

    /// Resets the dash pattern to empty (which is a solid line)
    #[inline]
    pub (super) fn tes_new_dash_pattern(&mut self) {
        self.core.sync(|core| core.layer(self.current_layer).state.stroke_settings.dash_pattern = vec![]);
    }

    /// Adds a dash to the current dash pattern
    #[inline]
    pub (super) fn tes_dash_length(&mut self, dash_length: f32) {
        self.core.sync(|core| core.layer(self.current_layer).state.stroke_settings.dash_pattern.push(dash_length));
    }

    /// Sets the offset for the dash pattern
    #[inline]
    pub (super) fn tes_dash_offset(&mut self, offset: f32) {
        self.core.sync(|core| core.layer(self.current_layer).state.stroke_settings.dash_offset = offset);
    }

    /// Set the fill color
    #[inline]
    pub (super) fn tes_fill_color(&mut self, color: canvas::Color) {
        self.core.sync(|core| core.layer(self.current_layer).state.fill_color = FillState::Color(Self::render_color(color)));
    }

    /// Set a fill texture
    #[inline]
    pub (super) fn tes_fill_texture(&mut self, namespace_id: usize, texture_id: canvas::TextureId, (x1, y1): (f32, f32), (x2, y2): (f32, f32)) {
        self.core.sync(|core| {
            // Check that the texture is ready for rendering (this also commits it at the point it's selected)
            let render_texture  = core.texture_for_rendering(namespace_id, texture_id);
            if let Some(render_texture) = render_texture {
                // Choose this texture
                let alpha               = core.texture_alpha.get(&(namespace_id, texture_id)).cloned().unwrap_or(1.0);
                let layer               = core.layer(self.current_layer);

                layer.state.fill_color  = FillState::texture_fill(render_texture, texture_id, x1, y1, x2, y2, alpha)
            }
        });
    }

    /// Set a fill gradient
    #[inline]
    pub (super) fn tes_fill_gradient(&mut self, namespace_id: usize, gradient_id: canvas::GradientId, (x1, y1): (f32, f32), (x2, y2): (f32, f32)) {
        self.core.sync(|core| {
            // Check that the texture is ready for rendering (this also commits it at the point it's selected)
            let render_gradient  = core.gradient_for_rendering(namespace_id, gradient_id);
            if let Some(render_gradient) = render_gradient {
                // Choose this gradient
                let layer               = core.layer(self.current_layer);

                layer.state.fill_color  = FillState::linear_gradient_fill(render_gradient, gradient_id, x1, y1, x2, y2);
            }
        });
    }

    /// Transforms the existing fill
    #[inline]
    pub (super) fn tes_fill_transform(&mut self, transform: canvas::Transform2D) {
        self.core.sync(|core| {
            let layer               = core.layer(self.current_layer);

            let transform           = transform.invert().unwrap_or_else(|| canvas::Transform2D::identity());
            layer.state.fill_color  = layer.state.fill_color.transform(&transform);
        });
    }

    // Set the line color
    #[inline]
    pub (super) fn tes_stroke_color(&mut self, color: canvas::Color) {
        self.core.sync(|core| core.layer(self.current_layer).state.stroke_settings.stroke_color = Self::render_color(color));
    }

    /// Set how future renderings are blended with one another
    pub (super) fn tes_blend_mode(&mut self, blend_mode: canvas::BlendMode) {
        self.core.sync(|core| {
            use canvas::BlendMode::*;
            core.layer(self.current_layer).state.blend_mode = blend_mode;

            let blend_mode = match blend_mode {
                SourceOver      => render::BlendMode::SourceOver,
                DestinationOver => render::BlendMode::DestinationOver,
                DestinationOut  => render::BlendMode::DestinationOut,

                SourceIn        => render::BlendMode::SourceIn,
                SourceOut       => render::BlendMode::SourceOut,
                DestinationIn   => render::BlendMode::DestinationIn,
                SourceAtop      => render::BlendMode::SourceATop,
                DestinationAtop => render::BlendMode::DestinationATop,

                Multiply        => render::BlendMode::Multiply,
                Screen          => render::BlendMode::Screen,

                // TODO: these are not supported yet (they might require explicit shader support)
                Darken          => render::BlendMode::SourceOver,
                Lighten         => render::BlendMode::SourceOver,
            };

            core.layer(self.current_layer).render_order.push(RenderEntity::SetBlendMode(blend_mode));
        });
    }
}