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
    /// Suspends rendering to the display until the next 'ShowFrame'
    ///
    /// The renderer may perform tessellation or rendering in the background after 'StartFrame' but won't
    /// commit anything to the visible frame buffer until 'ShowFrame' is hit. If 'StartFrame' is nested,
    /// then the frame won't be displayed until 'ShowFrame' has been requested at least that many times.
    ///
    /// The frame state persists across a 'ClearCanvas'
    fn start_frame(&mut self);

    /// Displays any requested queued after 'StartFrame'
    fn show_frame(&mut self);

    /// Resets the frame count back to 0 (for when regenerating the state of a canvas)
    fn reset_frame(&mut self);



    /// Begins a new path
    fn new_path(&mut self);

    /// Move to a new point in the current path (paths should always begin with a move instruction, and moves can define subpaths)
    fn move_to(&mut self, x: f32, y: f32);

    /// Adds a line to the current path
    fn line_to(&mut self, x: f32, y: f32);

    /// Adds a bezier curve to the current path
    fn bezier_curve_to(&mut self, x: f32, y: f32, cp1_x: f32, cp1_y: f32, cp2_x: f32, cp2_y: f32);

    /// Closes the current path (adds a line to the last move point)
    fn close_path(&mut self);

    /// Fills the currently defined path
    fn fill(&mut self);

    /// Draws a line around the currently defined path
    fn stroke(&mut self);

    /// Sets the line width for the next stroke() operation
    fn line_width(&mut self, width: f32);

    /// Sets the line width for the next stroke() operation in device pixels
    fn line_width_pixels(&mut self, width: f32);

    /// Sets the line join style for the next stroke() operation
    fn line_join(&mut self, join: LineJoin);

    /// Sets the style of the start and end cap of the next line drawn by the stroke() operation
    fn line_cap(&mut self, cap: LineCap);

    /// Sets the winding rule used to determine if an internal subpath should be filled or empty
    fn winding_rule(&mut self, winding_rule: WindingRule);

    /// Starts defining a new dash pattern
    fn new_dash_pattern(&mut self);

    /// Adds a dash of the specified length to the dash pattern (alternating between drawing and gap lengths)
    fn dash_length(&mut self, length: f32);

    /// Sets the offset for where the dash pattern starts at the next stroke
    fn dash_offset(&mut self, offset: f32);

    /// Sets the colour of the next fill() operation
    fn fill_color(&mut self, col: Color);

    /// Sets the texture to use for the next fill() operation
    fn fill_texture(&mut self, texture_id: TextureId, x1: f32, y1: f32, x2: f32, y2: f32);

    /// Sets the gradient to use for the next fill() operation
    fn fill_gradient(&mut self, gradient_id: GradientId, x1: f32, y1: f32, x2: f32, y2: f32);

    /// Applies a transformation to the fill texture or gradient
    fn fill_transform(&mut self, transform: Transform2D);

    /// Sets the colour to use for the next stroke() operation
    fn stroke_color(&mut self, col: Color);

    /// Sets the blend mode of the next fill or stroke operation
    fn blend_mode(&mut self, mode: BlendMode);

    /// Reset the canvas transformation to the identity transformation (so that the y axis goes from -1 to 1)
    fn identity_transform(&mut self);

    /// Sets a transformation such that:
    /// (0,0) is the center point of the canvas
    /// (0,height/2) is the top of the canvas
    /// Pixels are square
    fn canvas_height(&mut self, height: f32);

    /// Moves a particular region to the center of the canvas (coordinates are minx, miny, maxx, maxy)
    fn center_region(&mut self, minx: f32, miny: f32, maxx: f32, maxy: f32);

    /// Multiply a 2D transform by the current transformation
    fn transform(&mut self, transform: Transform2D);

    /// Removes the current clipping path
    fn unclip(&mut self);

    /// Sets the current path as the clipping path
    fn clip(&mut self);

    /// Stores the current contents of the canvas in a background buffer
    fn store(&mut self);

    /// Restores the contents of the canvas from the background buffer
    fn restore(&mut self);

    /// Releases the memory allocated by the last store() operation
    fn free_stored_buffer(&mut self);

    /// Stores the current state of the canvas (line width, fill colour, etc)
    fn push_state(&mut self);

    /// Restore a state previously pushed
    ///
    /// This will restore the line width (and the other stroke settings), stroke colour, current path, fill colour,
    /// winding rule, sprite settings and blend settings.
    ///
    /// The currently selected layer is not affected by this operation.
    fn pop_state(&mut self);

    /// Clears the canvas entirely to a background colour, and removes any stored resources (layers, sprites, fonts, textures)
    fn clear_canvas(&mut self, color: Color);



    /// Selects a particular layer for drawing
    /// Layer 0 is selected initially. Layers are drawn in order starting from 0.
    /// Layer IDs don't have to be sequential.
    fn layer(&mut self, layer_id: LayerId);

    /// Sets how a particular layer is blended with the underlying layer
    fn layer_blend(&mut self, layer_id: LayerId, blend_mode: BlendMode);

    /// Clears the current layer
    fn clear_layer(&mut self);

    /// Clears all of the layers (without resetting any other resources, as clear_canvas does)
    fn clear_all_layers(&mut self);

    /// Exchanges the contents of two layers in the drawing
    fn swap_layers(&mut self, layer1: LayerId, layer2: LayerId);



    /// Selects a particular sprite for drawing
    ///
    /// Future drawing actions are sent to this sprite: use something like `Layer(0)` to start drawing
    /// to a layer again.
    ///
    /// Sprites can be repeatedly re-rendered with a single command and their appearance may be
    /// cached for efficiency. Actions that affect the whole canvas or layers are not permitted in
    /// sprites.
    fn sprite(&mut self, sprite_id: SpriteId);

    /// Releases the resources used by the current sprite
    fn clear_sprite(&mut self);

    /// Adds a sprite transform to the next sprite drawing operation
    fn sprite_transform(&mut self, transform: SpriteTransform);

    /// Renders a sprite with a set of transformations
    fn draw_sprite(&mut self, sprite_id: SpriteId);



    /// Loads font data into the canvas for a particular font ID
    fn define_font_data(&mut self, font_id: FontId, font_data: Arc<CanvasFontFace>);

    /// Sets the size that text in the specified font will be rendered at
    fn set_font_size(&mut self, font_id: FontId, size: f32);

    /// Draws a text string using a font
    fn draw_text(&mut self, font_id: FontId, text: String, baseline_x: f32, baseline_y: f32);

    /// Draws specific glyphs from a font
    fn draw_glyphs(&mut self, font_id: FontId, glyphs: Vec<GlyphPosition>);

    /// Starts laying out a line of text
    fn begin_line_layout(&mut self, x: f32, y: f32, align: TextAlignment);

    /// Adds text to the current line layout
    fn layout_text(&mut self, font_id: FontId, text: String);

    /// Finishes laying out text and renders the result
    fn draw_text_layout(&mut self);



    /// Creates a new texture that can be used with fill_texture of the specified width and height
    fn create_texture(&mut self, texture_id: TextureId, width: u32, height: u32, format: TextureFormat);

    /// Releases the memory allocated to a particular texture
    fn free_texture(&mut self, texture_id: TextureId);

    /// Sets the bitmap data for a texture, in the format specified by the call to create_texture()
    fn set_texture_bytes(&mut self, texture_id: TextureId, x: u32, y: u32, width: u32, height: u32, bytes: Arc<Vec<u8>>);

    /// Applies an alpha value to a texture
    fn set_texture_fill_alpha(&mut self, texture_id: TextureId, alpha: f32);



    /// Defines a new gradient with a colour at stop position 0.0. Gradients can be used via fill_gradient()
    fn create_gradient(&mut self, gradient_id: GradientId, initial_color: Color);

    /// Adds a new colour stop to a texture
    fn gradient_stop(&mut self, gradient_id: GradientId, pos: f32, color: Color);



    /// Sends a single drawing instruction to this graphics context
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
            FillTransform(transform)                                    => self.fill_transform(transform),
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
            ClearAllLayers                                              => self.clear_all_layers(),
            SwapLayers(layer1, layer2)                                  => self.swap_layers(layer1, layer2),
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

            Gradient(gradient_id, GradientOp::Create(initial_color))                => self.create_gradient(gradient_id, initial_color),
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
    #[inline] fn fill_transform(&mut self, transform: Transform2D)                              { self.push(Draw::FillTransform(transform)); }
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
    #[inline] fn clear_all_layers(&mut self)                                                    { self.push(Draw::ClearAllLayers); }
    #[inline] fn swap_layers(&mut self, layer1: LayerId, layer2: LayerId)                       { self.push(Draw::SwapLayers(layer1, layer2)); }
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

    #[inline] fn create_gradient(&mut self, gradient_id: GradientId, initial_color: Color)                                      { self.push(Draw::Gradient(gradient_id, GradientOp::Create(initial_color))); }
    #[inline] fn gradient_stop(&mut self, gradient_id: GradientId, pos: f32, color: Color)                                      { self.push(Draw::Gradient(gradient_id, GradientOp::AddStop(pos, color))); }

    #[inline]
    fn draw(&mut self, d: Draw) {
        self.push(d);
    }
}
