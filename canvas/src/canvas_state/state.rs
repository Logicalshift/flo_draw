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

use std::collections::*;
use std::sync::*;

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

    layers_and_sprites: HashMap<DrawingTarget, Vec<CanvasEntity>>,
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
            layers_and_sprites: HashMap::new(), 
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
            Draw::Fill                                  => todo!(),
            Draw::Stroke                                => todo!(),
            Draw::LineWidth(_)                          => todo!(),
            Draw::LineWidthPixels(_)                    => todo!(),
            Draw::LineJoin(line_join)                   => todo!(),
            Draw::LineCap(line_cap)                     => todo!(),
            Draw::NewDashPattern                        => todo!(),
            Draw::DashLength(_)                         => todo!(),
            Draw::DashOffset(_)                         => todo!(),
            Draw::FillColor(color)                      => todo!(),
            Draw::FillTexture(texture_id, _, _)         => todo!(),
            Draw::FillGradient(gradient_id, _, _)       => todo!(),
            Draw::FillTransform(transform2_d)           => todo!(),
            Draw::StrokeColor(color)                    => todo!(),
            Draw::WindingRule(winding_rule)             => todo!(),
            Draw::BlendMode(blend_mode)                 => todo!(),
            Draw::IdentityTransform                     => todo!(),
            Draw::CanvasHeight(_)                       => todo!(),
            Draw::CenterRegion(_, _)                    => todo!(),
            Draw::MultiplyTransform(transform2_d)       => todo!(),
            Draw::Unclip                                => todo!(),
            Draw::Clip                                  => todo!(),
            Draw::Store                                 => todo!(),
            Draw::Restore                               => todo!(),
            Draw::FreeStoredBuffer                      => todo!(),
            Draw::PushState                             => todo!(),
            Draw::PopState                              => todo!(),
            Draw::ClearCanvas(color)                    => todo!(),
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

    #[inline] fn start_frame(&mut self) { self.frame_count += 1; }
    #[inline] fn show_frame(&mut self) { self.frame_count -= 1; }
    #[inline] fn reset_frame(&mut self) { self.frame_count = 0; }
    #[inline] fn path_op(&mut self, op: PathOp) { self.current_path.draw(op); }
}
