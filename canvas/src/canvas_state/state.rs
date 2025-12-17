use super::path::*;
use super::text::*;
use super::brush::*;
use super::share::*;
use super::entity::*;
use super::texture::*;
use super::gradient::*;
use super::drawing_target::*;

use crate::draw::*;
use crate::path::*;
use crate::color::*;
use crate::gradient::*;
use crate::font_face::*;
use crate::transform2d::*;

use std::collections::*;
use std::sync::*;

// TODO: in canvas_renderer, pushstate and popstate are per-layer
// TODO: store/restore are supposed to clip against the current clipping path (but this is not actually happening in canvas_renderer)

///
/// Contains the current state of a canvas (so its drawing instructions can be replicated or relayed)
///
#[derive(Clone)]
pub struct CanvasState {
    keep_textures:      bool,
    frame_count:        usize,
    background:         Color,

    current_brush:      CanvasShared<CanvasBrush>,
    state_stack:        Vec<Arc<CanvasBrush>>,

    canvas_clear_count: usize,
    layers_and_sprites: HashMap<DrawingTarget, Vec<CanvasEntity>>,
    stored_state:       HashMap<DrawingTarget, usize>,
    clear_count:        HashMap<DrawingTarget, usize>,
    textures:           HashMap<TextureId, CanvasShared<CanvasTexture>>,
    gradients:          HashMap<GradientId, CanvasShared<CanvasGradient>>,
    fonts:              HashMap<FontId, Arc<CanvasFontFace>>,

    current_path:       CanvasPath,
    current_text:       Vec<CanvasTextLayout>,
}

impl Default for CanvasState {
    ///
    /// Creates a canvas in the default state
    ///
    fn default() -> Self {
        CanvasState { 
            keep_textures:      true,
            frame_count:        0,
            background:         Color::Rgba(1.0, 1.0, 1.0, 1.0), 
            current_brush:      CanvasShared::new(CanvasBrush::default()), 
            state_stack:        vec![], 
            canvas_clear_count: 0,
            layers_and_sprites: HashMap::new(), 
            stored_state:       HashMap::new(),
            clear_count:        HashMap::new(), 
            textures:           HashMap::new(), 
            gradients:          HashMap::new(), 
            fonts:              HashMap::new(), 
            current_path:       CanvasPath::default(), 
            current_text:       vec![],
        }
    }
}

impl CanvasState {
    ///
    /// Updates this state with a drawing action
    ///
    pub fn draw(&mut self, action: Draw) {
        match action {
            Draw::StartFrame                            => self.start_frame(),
            Draw::ShowFrame                             => self.show_frame(),
            Draw::ResetFrame                            => self.reset_frame(),
            Draw::Path(path_op)                         => self.path_op(path_op),
            Draw::Fill                                  => self.fill(),
            Draw::Stroke                                => self.stroke(),
            Draw::LineWidth(w)                          => self.line_width(w),
            Draw::LineWidthPixels(w)                    => self.line_width_pixels(w),
            Draw::LineJoin(line_join)                   => self.line_join(line_join),
            Draw::LineCap(line_cap)                     => self.line_cap(line_cap),
            Draw::NewDashPattern                        => self.new_dash_pattern(),
            Draw::DashLength(length)                    => self.dash_length(length),
            Draw::DashOffset(offset)                    => self.dash_offset(offset),
            Draw::FillColor(color)                      => self.fill_color(color),
            Draw::FillTexture(texture_id, min, max)     => self.fill_texture(texture_id, min, max),
            Draw::FillGradient(gradient_id, min, max)   => self.fill_gradient(gradient_id, min, max),
            Draw::FillTransform(transform)              => self.fill_transform(transform),
            Draw::StrokeColor(color)                    => self.stroke_color(color),
            Draw::WindingRule(winding_rule)             => self.winding_rule(winding_rule),
            Draw::BlendMode(blend_mode)                 => self.blend_mode(blend_mode),
            Draw::IdentityTransform                     => self.identity_transform(),
            Draw::CanvasHeight(height)                  => self.canvas_height(height),
            Draw::CenterRegion(min, max)                => self.center_region(min, max),
            Draw::MultiplyTransform(transform)          => self.multiply_transform(transform),
            Draw::Unclip                                => self.unclip(),
            Draw::Clip                                  => self.clip(),
            Draw::Store                                 => self.store(),
            Draw::Restore                               => self.restore(),
            Draw::FreeStoredBuffer                      => self.free_stored_buffer(),
            Draw::PushState                             => self.push_state(),
            Draw::PopState                              => self.pop_state(),
            Draw::ClearCanvas(color)                    => self.clear_canvas(color),
            Draw::Layer(layer_id)                       => todo!(),
            Draw::LayerBlend(layer_id, blend_mode)      => todo!(),
            Draw::ClearLayer                            => todo!(),
            Draw::ClearAllLayers                        => todo!(),
            Draw::SwapLayers(layer_id, layer_id1)       => todo!(),
            Draw::Sprite(sprite_id)                     => todo!(),
            Draw::ClearSprite                           => todo!(),
            Draw::SpriteTransform(sprite_transform)     => todo!(),
            Draw::DrawSprite(sprite_id)                 => todo!(),
            Draw::Texture(texture_id, texture_op)       => todo!(),
            Draw::Font(font_id, font_op)                => todo!(),
            Draw::BeginLineLayout(_, _, text_alignment) => todo!(),
            Draw::DrawLaidOutText                       => todo!(),
            Draw::DrawText(font_id, _, _, _)            => todo!(),
            Draw::Gradient(gradient_id, gradient_op)    => todo!(),
        }
    }

    #[inline] fn start_frame(&mut self)         { self.frame_count += 1; }
    #[inline] fn show_frame(&mut self)          { self.frame_count -= 1; }
    #[inline] fn reset_frame(&mut self)         { self.frame_count = 0; }
    #[inline] fn path_op(&mut self, op: PathOp) { self.current_path.draw(op); }

    #[inline] fn line_width(&mut self, width: f32)          { self.current_brush.get_mut().line_width = LineWidth::Width(width); }
    #[inline] fn line_width_pixels(&mut self, width: f32)   { self.current_brush.get_mut().line_width = LineWidth::Width(width); }
    #[inline] fn line_join(&mut self, join: LineJoin)       { self.current_brush.get_mut().line_join = join; }
    #[inline] fn line_cap(&mut self, cap: LineCap)          { self.current_brush.get_mut().line_cap = cap; }
    #[inline] fn new_dash_pattern(&mut self)                { self.current_brush.get_mut().dash_pattern = vec![]; }
    #[inline] fn dash_length(&mut self, len: f32)           { self.current_brush.get_mut().dash_pattern.push(len); }
    #[inline] fn dash_offset(&mut self, offset: f32)        { self.current_brush.get_mut().dash_offset = offset; }
    #[inline] fn stroke_color(&mut self, color: Color)      { self.current_brush.get_mut().stroke_color = color; }

    #[inline] fn fill_color(&mut self, color: Color)                                                    { self.current_brush.get_mut().fill = FillState::Color(color); }
    #[inline] fn fill_texture(&mut self, texture_id: TextureId, min: (f32, f32), max: (f32, f32))       { self.current_brush.get_mut().fill = FillState::Texture(texture_id, min, max); self.current_brush.get_mut().fill_transform = Transform2D::identity(); }
    #[inline] fn fill_gradient(&mut self, gradient_id: GradientId, min: (f32, f32), max: (f32, f32))    { self.current_brush.get_mut().fill = FillState::Gradient(gradient_id, min, max); self.current_brush.get_mut().fill_transform = Transform2D::identity(); }
    #[inline] fn fill_transform(&mut self, transform: Transform2D)                                      { self.current_brush.get_mut().fill_transform = self.current_brush.get_mut().fill_transform * transform; }
    #[inline] fn winding_rule(&mut self, winding_rule: WindingRule)                                     { self.current_brush.get_mut().winding_rule = winding_rule; }

    #[inline] fn blend_mode(&mut self, blend_mode: BlendMode)               { self.current_brush.get_mut().blend_mode = blend_mode; }
    #[inline] fn identity_transform(&mut self)                              { self.current_brush.get_mut().canvas_height = 2.0; self.current_brush.get_mut().center_region = None; self.current_brush.get_mut().multiply_transform = Transform2D::identity(); }
    #[inline] fn canvas_height(&mut self, height: f32)                      { self.current_brush.get_mut().canvas_height = height; self.current_brush.get_mut().center_region = None; self.current_brush.get_mut().multiply_transform = Transform2D::identity(); }
    #[inline] fn center_region(&mut self, min: (f32, f32), max: (f32, f32)) { self.current_brush.get_mut().center_region = Some((min, max)); }
    #[inline] fn multiply_transform(&mut self, transform: Transform2D)      { self.current_brush.get_mut().multiply_transform = self.current_brush.get_mut().multiply_transform * transform; }

    #[inline] fn unclip(&mut self)      { self.current_brush.get_mut().clip_path.clear(); }
    #[inline] fn clip(&mut self)        { self.current_brush.get_mut().clip_path.push(self.current_path.clone()); }

    #[inline] fn push_state(&mut self)  { self.state_stack.push(self.current_brush.into_arc()); }
    #[inline] fn pop_state(&mut self)   { if let Some(state) = self.state_stack.pop() { self.current_brush = CanvasShared::from_arc(state); } }

    #[inline] fn fill(&mut self) {
        let mut brush       = self.current_brush.shared();
        let target          = (*brush).drawing_target();
        let path            = self.current_path.clone();
        let fill_texture    = brush.fill_texture().and_then(|texture_id| self.textures.get_mut(&texture_id)).map(|texture| texture.into_arc());
        let fill_gradient   = brush.fill_gradient().and_then(|gradient_id| self.gradients.get_mut(&gradient_id)).map(|gradient| gradient.into_arc());

        let entity          = CanvasEntity::fill(brush.into_arc(), path, fill_texture, fill_gradient);

        self.layers_and_sprites.entry(target)
            .or_insert_with(|| vec![])
            .push(entity);
    }

    #[inline] fn stroke(&mut self) {
        let mut brush   = self.current_brush.shared();
        let target      = (*brush).drawing_target();
        let path        = self.current_path.clone();
        let entity      = CanvasEntity::stroke(brush.into_arc(), path);

        self.layers_and_sprites.entry(target)
            .or_insert_with(|| vec![])
            .push(entity);
    }

    fn clear_canvas(&mut self, new_background: Color) {
        self.canvas_clear_count += 1;
        self.layers_and_sprites = HashMap::new();
        self.stored_state       = HashMap::new();
        self.clear_count        = HashMap::new();
        self.background         = new_background;
        self.state_stack        = vec![];
        self.current_path       = CanvasPath::default();

        let brush = self.current_brush.get_mut();

        brush.target                = DrawingTarget::Layer(LayerId(0));
        brush.fill                  = FillState::Color(Color::Rgba(0.0, 0.0, 0.0, 1.0));
        brush.multiply_transform    = Transform2D::identity();
    }

    #[inline] fn store(&mut self) {
        let drawing_target      = self.current_brush.get().drawing_target();
        let drawing_position    = self.layers_and_sprites.get(&drawing_target).map(|layer| layer.len()).unwrap_or(0);

        self.stored_state.insert(drawing_target, drawing_position);
    }

    #[inline] fn restore(&mut self) {
        let drawing_target = self.current_brush.get().drawing_target();

        if let Some(drawing_position) = self.stored_state.get(&drawing_target).copied() {
            if let Some(layer) = self.layers_and_sprites.get_mut(&drawing_target) {
                layer.truncate(drawing_position);
            }
        }
    }

    #[inline] fn free_stored_buffer(&mut self) {
        let drawing_target = self.current_brush.get().drawing_target();
        self.stored_state.remove(&drawing_target);
    }
}
