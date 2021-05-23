use crate::draw::*;
use crate::path::*;
use crate::font::*;
use crate::color::*;
use crate::texture::*;
use crate::gradient::*;
use crate::font_face::*;
use crate::transform2d::*;

use std::sync::*;

///
/// A graphics context provides the basic set of graphics actions that can be performed
///
pub trait GraphicsContext {
    fn start_frame(&mut self);
    fn show_frame(&mut self);
    fn reset_frame(&mut self);

    fn new_path(&mut self);
    fn move_to(&mut self, x: f32, y: f32);
    fn line_to(&mut self, x: f32, y: f32);
    fn bezier_curve_to(&mut self, x: f32, y: f32, cp1_x: f32, cp1_y: f32, cp2_x: f32, cp2_y: f32);
    fn close_path(&mut self);
    fn fill(&mut self);
    fn stroke(&mut self);
    fn line_width(&mut self, width: f32);
    fn line_width_pixels(&mut self, width: f32);
    fn line_join(&mut self, join: LineJoin);
    fn line_cap(&mut self, cap: LineCap);
    fn winding_rule(&mut self, winding_rule: WindingRule);
    fn new_dash_pattern(&mut self);
    fn dash_length(&mut self, length: f32);
    fn dash_offset(&mut self, offset: f32);
    fn fill_color(&mut self, col: Color);
    fn fill_texture(&mut self, texture_id: TextureId, x: f32, y: f32, width: f32, height: f32);
    fn fill_gradient(&mut self, gradient_id: GradientId, x1: f32, y1: f32, x2: f32, y2: f32);
    fn stroke_color(&mut self, col: Color);
    fn blend_mode(&mut self, mode: BlendMode);
    fn identity_transform(&mut self);
    fn canvas_height(&mut self, height: f32);
    fn center_region(&mut self, minx: f32, miny: f32, maxx: f32, maxy: f32);
    fn transform(&mut self, transform: Transform2D);
    fn unclip(&mut self);
    fn clip(&mut self);
    fn store(&mut self);
    fn restore(&mut self);
    fn free_stored_buffer(&mut self);
    fn push_state(&mut self);
    fn pop_state(&mut self);
    fn clear_canvas(&mut self, color: Color);

    fn layer(&mut self, layer_id: LayerId);
    fn layer_blend(&mut self, layer_id: LayerId, blend_mode: BlendMode);
    fn clear_layer(&mut self);

    fn sprite(&mut self, sprite_id: SpriteId);
    fn clear_sprite(&mut self);
    fn sprite_transform(&mut self, transform: SpriteTransform);
    fn draw_sprite(&mut self, sprite_id: SpriteId);

    fn define_font_data(&mut self, font_id: FontId, font_data: Arc<CanvasFontFace>);
    fn set_font_size(&mut self, font_id: FontId, size: f32);
    fn draw_text(&mut self, font_id: FontId, text: String, baseline_x: f32, baseline_y: f32);
    fn draw_glyphs(&mut self, font_id: FontId, glyphs: Vec<GlyphPosition>);
    fn begin_line_layout(&mut self, x: f32, y: f32, align: TextAlignment);
    fn layout_text(&mut self, font_id: FontId, text: String);
    fn draw_text_layout(&mut self);

    fn create_texture(&mut self, texture_id: TextureId, width: u32, height: u32, format: TextureFormat);
    fn free_texture(&mut self, texture_id: TextureId);
    fn set_texture_bytes(&mut self, texture_id: TextureId, x: u32, y: u32, width: u32, height: u32, bytes: Arc<Vec<u8>>);
    fn set_texture_fill_alpha(&mut self, texture_id: TextureId, alpha: f32);

    fn new_gradient(&mut self, gradient_id: GradientId, initial_color: Color);
    fn gradient_stop(&mut self, gradient_id: GradientId, pos: f32, color: Color);

    fn draw(&mut self, d: Draw) {
        use self::Draw::*;
        use self::PathOp::*;

        match d {
            StartFrame                                                  => self.start_frame(),
            ShowFrame                                                   => self.show_frame(),
            ResetFrame                                                  => self.reset_frame(),
            Path(NewPath)                                               => self.new_path(),
            Path(Move(x, y))                                            => self.move_to(x, y),
            Path(Line(x, y) )                                           => self.line_to(x, y),
            Path(BezierCurve(((cp1x, cp1y), (cp2x, cp2y)), (x1, y1)))   => self.bezier_curve_to(x1, y1, cp1x, cp1y, cp2x, cp2y),
            Path(ClosePath)                                             => self.close_path(),
            Fill                                                        => self.fill(),
            Stroke                                                      => self.stroke(),
            LineWidth(width)                                            => self.line_width(width),
            LineWidthPixels(width)                                      => self.line_width_pixels(width),
            LineJoin(join)                                              => self.line_join(join),
            LineCap(cap)                                                => self.line_cap(cap),
            WindingRule(rule)                                           => self.winding_rule(rule),
            NewDashPattern                                              => self.new_dash_pattern(),
            DashLength(dash_length)                                     => self.dash_length(dash_length),
            DashOffset(dash_offset)                                     => self.dash_offset(dash_offset),
            FillColor(col)                                              => self.fill_color(col),
            FillTexture(texture, (x1, y1), (x2, y2))                    => self.fill_texture(texture, x1, y1, x2, y2),
            FillGradient(gradient, (x1, y1), (x2, y2))                  => self.fill_gradient(gradient, x1, y1, x2, y2),
            StrokeColor(col)                                            => self.stroke_color(col),
            BlendMode(blendmode)                                        => self.blend_mode(blendmode),
            IdentityTransform                                           => self.identity_transform(),
            CanvasHeight(height)                                        => self.canvas_height(height),
            CenterRegion((minx, miny), (maxx, maxy))                    => self.center_region(minx, miny, maxx, maxy),
            MultiplyTransform(transform)                                => self.transform(transform),
            Unclip                                                      => self.unclip(),
            Clip                                                        => self.clip(),
            Store                                                       => self.store(),
            Restore                                                     => self.restore(),
            FreeStoredBuffer                                            => self.free_stored_buffer(),
            PushState                                                   => self.push_state(),
            PopState                                                    => self.pop_state(),
            ClearCanvas(color)                                          => self.clear_canvas(color),
            Layer(layer_id)                                             => self.layer(layer_id),
            LayerBlend(layer_id, blend_mode)                            => self.layer_blend(layer_id, blend_mode),
            ClearLayer                                                  => self.clear_layer(),
            Sprite(sprite_id)                                           => self.sprite(sprite_id),
            ClearSprite                                                 => self.clear_sprite(),
            SpriteTransform(transform)                                  => self.sprite_transform(transform),
            DrawSprite(sprite_id)                                       => self.draw_sprite(sprite_id),

            Font(font_id, FontOp::UseFontDefinition(font_data))                     => self.define_font_data(font_id, font_data),
            Font(font_id, FontOp::FontSize(font_size))                              => self.set_font_size(font_id, font_size),
            Font(font_id, FontOp::LayoutText(text))                                 => self.layout_text(font_id, text),
            Font(font_id, FontOp::DrawGlyphs(glyphs))                               => self.draw_glyphs(font_id, glyphs),
            DrawText(font_id, string, x, y)                                         => self.draw_text(font_id, string, x, y),
            BeginLineLayout(x, y, alignment)                                        => self.begin_line_layout(x, y, alignment),
            DrawLaidOutText                                                         => self.draw_text_layout(),
            
            Texture(texture_id, TextureOp::Create(width, height, format))           => self.create_texture(texture_id, width, height, format),
            Texture(texture_id, TextureOp::Free)                                    => self.free_texture(texture_id),
            Texture(texture_id, TextureOp::SetBytes(x, y, w, h, bytes))             => self.set_texture_bytes(texture_id, x, y, w, h, bytes),
            Texture(texture_id, TextureOp::FillTransparency(alpha))                 => self.set_texture_fill_alpha(texture_id, alpha),

            Gradient(gradient_id, GradientOp::New(initial_color))                   => self.new_gradient(gradient_id, initial_color),
            Gradient(gradient_id, GradientOp::AddStop(pos, color))                  => self.gradient_stop(gradient_id, pos, color)
        }
    }
}

///
/// A Vec<Draw> can be treated as a target for graphics primitives (just pushing the appropriate draw instructions)
///
impl GraphicsContext for Vec<Draw> {
    #[inline] fn start_frame(&mut self)                                                         { self.push(Draw::StartFrame); }
    #[inline] fn show_frame(&mut self)                                                          { self.push(Draw::ShowFrame); }
    #[inline] fn reset_frame(&mut self)                                                         { self.push(Draw::ResetFrame); }
    #[inline] fn new_path(&mut self)                                                            { self.push(Draw::Path(PathOp::NewPath)); }
    #[inline] fn move_to(&mut self, x: f32, y: f32)                                             { self.push(Draw::Path(PathOp::Move(x, y))); }
    #[inline] fn line_to(&mut self, x: f32, y: f32)                                             { self.push(Draw::Path(PathOp::Line(x, y))); }
    #[inline] fn bezier_curve_to(&mut self, x1: f32, y1: f32, cp1x: f32, cp1y: f32, cp2x: f32, cp2y: f32) { 
        self.push(Draw::Path(PathOp::BezierCurve(((cp1x, cp1y), (cp2x, cp2y)), (x1, y1)))); 
    }
    #[inline] fn close_path(&mut self)                                                          { self.push(Draw::Path(PathOp::ClosePath)); }
    #[inline] fn fill(&mut self)                                                                { self.push(Draw::Fill); }
    #[inline] fn stroke(&mut self)                                                              { self.push(Draw::Stroke); }
    #[inline] fn line_width(&mut self, width: f32)                                              { self.push(Draw::LineWidth(width)); }
    #[inline] fn line_width_pixels(&mut self, width: f32)                                       { self.push(Draw::LineWidthPixels(width)); }
    #[inline] fn line_join(&mut self, join: LineJoin)                                           { self.push(Draw::LineJoin(join)); }
    #[inline] fn line_cap(&mut self, cap: LineCap)                                              { self.push(Draw::LineCap(cap)); }
    #[inline] fn winding_rule(&mut self, rule: WindingRule)                                     { self.push(Draw::WindingRule(rule)); }
    #[inline] fn new_dash_pattern(&mut self)                                                    { self.push(Draw::NewDashPattern); }
    #[inline] fn dash_length(&mut self, length: f32)                                            { self.push(Draw::DashLength(length)); }
    #[inline] fn dash_offset(&mut self, offset: f32)                                            { self.push(Draw::DashOffset(offset)); }
    #[inline] fn fill_color(&mut self, col: Color)                                              { self.push(Draw::FillColor(col)); }
    #[inline] fn fill_texture(&mut self, t: TextureId, x1: f32, y1: f32, x2: f32, y2: f32)      { self.push(Draw::FillTexture(t, (x1, y1), (x2, y2))); }
    #[inline] fn fill_gradient(&mut self, g: GradientId, x1: f32, y1: f32, x2: f32, y2: f32)    { self.push(Draw::FillGradient(g, (x1, y1), (x2, y2))); }
    #[inline] fn stroke_color(&mut self, col: Color)                                            { self.push(Draw::StrokeColor(col)); }
    #[inline] fn blend_mode(&mut self, mode: BlendMode)                                         { self.push(Draw::BlendMode(mode)); }
    #[inline] fn identity_transform(&mut self)                                                  { self.push(Draw::IdentityTransform); }
    #[inline] fn canvas_height(&mut self, height: f32)                                          { self.push(Draw::CanvasHeight(height)); }
    #[inline] fn center_region(&mut self, minx: f32, miny: f32, maxx: f32, maxy: f32)           { self.push(Draw::CenterRegion((minx, miny), (maxx, maxy))); }
    #[inline] fn transform(&mut self, transform: Transform2D)                                   { self.push(Draw::MultiplyTransform(transform)); }
    #[inline] fn unclip(&mut self)                                                              { self.push(Draw::Unclip); }
    #[inline] fn clip(&mut self)                                                                { self.push(Draw::Clip); }
    #[inline] fn store(&mut self)                                                               { self.push(Draw::Store); }
    #[inline] fn restore(&mut self)                                                             { self.push(Draw::Restore); }
    #[inline] fn free_stored_buffer(&mut self)                                                  { self.push(Draw::FreeStoredBuffer); }
    #[inline] fn push_state(&mut self)                                                          { self.push(Draw::PushState); }
    #[inline] fn pop_state(&mut self)                                                           { self.push(Draw::PopState); }
    #[inline] fn clear_canvas(&mut self, color: Color)                                          { self.push(Draw::ClearCanvas(color)); }
    #[inline] fn layer(&mut self, layer_id: LayerId)                                            { self.push(Draw::Layer(layer_id)); }
    #[inline] fn layer_blend(&mut self, layer_id: LayerId, blend_mode: BlendMode)               { self.push(Draw::LayerBlend(layer_id, blend_mode)); }
    #[inline] fn clear_layer(&mut self)                                                         { self.push(Draw::ClearLayer); }
    #[inline] fn sprite(&mut self, sprite_id: SpriteId)                                         { self.push(Draw::Sprite(sprite_id)); }
    #[inline] fn clear_sprite(&mut self)                                                        { self.push(Draw::ClearSprite); }
    #[inline] fn sprite_transform(&mut self, transform: SpriteTransform)                        { self.push(Draw::SpriteTransform(transform)); }
    #[inline] fn draw_sprite(&mut self, sprite_id: SpriteId)                                    { self.push(Draw::DrawSprite(sprite_id)); }

    #[inline] fn define_font_data(&mut self, font_id: FontId, font_data: Arc<CanvasFontFace>)                                   { self.push(Draw::Font(font_id, FontOp::UseFontDefinition(font_data))); }
    #[inline] fn set_font_size(&mut self, font_id: FontId, size: f32)                                                           { self.push(Draw::Font(font_id, FontOp::FontSize(size))); }
    #[inline] fn draw_text(&mut self, font_id: FontId, text: String, baseline_x: f32, baseline_y: f32)                          { self.push(Draw::DrawText(font_id, text, baseline_x, baseline_y)); }
    #[inline] fn draw_glyphs(&mut self, font_id: FontId, glyphs: Vec<GlyphPosition>)                                            { self.push(Draw::Font(font_id, FontOp::DrawGlyphs(glyphs))); }
    #[inline] fn begin_line_layout(&mut self, x: f32, y: f32, align: TextAlignment)                                             { self.push(Draw::BeginLineLayout(x, y, align)); }
    #[inline] fn layout_text(&mut self, font_id: FontId, text: String)                                                          { self.push(Draw::Font(font_id, FontOp::LayoutText(text))); }
    #[inline] fn draw_text_layout(&mut self)                                                                                    { self.push(Draw::DrawLaidOutText); }

    #[inline] fn create_texture(&mut self, texture_id: TextureId, w: u32, h: u32, format: TextureFormat)                        { self.push(Draw::Texture(texture_id, TextureOp::Create(w, h, format))); }
    #[inline] fn free_texture(&mut self, texture_id: TextureId)                                                                 { self.push(Draw::Texture(texture_id, TextureOp::Free)); }
    #[inline] fn set_texture_bytes(&mut self, texture_id: TextureId, x: u32, y: u32, w: u32, h: u32, bytes: Arc<Vec<u8>>)       { self.push(Draw::Texture(texture_id, TextureOp::SetBytes(x, y, w, h, bytes))); }
    #[inline] fn set_texture_fill_alpha(&mut self, texture_id: TextureId, alpha: f32)                                           { self.push(Draw::Texture(texture_id, TextureOp::FillTransparency(alpha))); }

    #[inline] fn new_gradient(&mut self, gradient_id: GradientId, initial_color: Color)                                         { self.push(Draw::Gradient(gradient_id, GradientOp::New(initial_color))); }
    #[inline] fn gradient_stop(&mut self, gradient_id: GradientId, pos: f32, color: Color)                                      { self.push(Draw::Gradient(gradient_id, GradientOp::AddStop(pos, color))); }

    #[inline]
    fn draw(&mut self, d: Draw) {
        self.push(d);
    }
}
